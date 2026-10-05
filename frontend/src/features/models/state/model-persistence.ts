/**
 * Loads the open model from the database and stores it there again, a couple
 * of seconds after it changes (ADR 0008).
 *
 * The database is the only copy: nothing is kept in the browser. What the
 * contract cannot express — shapes, colours, sizes, groups, edge curves — is
 * therefore not kept across a reload (see model-mapping).
 *
 * Provided per editor, next to `ModelEditorStore`.
 */
import { computed, DestroyRef, inject, Injectable, signal } from '@angular/core';
import { ApiError } from '../../../core/api/api-error';
import { OrgApi } from '../../../core/api/org-api';
import {
  fromRemote, PersistedModel, remoteFingerprint, testToInput, toModelInput,
} from './model-mapping';

/** How long after the last change a save starts. */
export const SAVE_DEBOUNCE_MS = 2000;
/** How long to wait before retrying while the backend is unreachable. */
const OFFLINE_RETRY_MS = 15_000;

export type SaveState =
  /** Nothing to do. */
  | 'idle'
  /** The stored copy is being fetched. */
  | 'loading'
  /** A change is waiting for the debounce. */
  | 'pending'
  | 'saving'
  | 'saved'
  /** The backend cannot be reached; the change is retried. */
  | 'offline'
  /** Someone else changed the model; autosave stops until the user chooses. */
  | 'conflict'
  | 'error';

/** What the database holds for the open model. */
interface Remote {
  version: number;
  featureId: string;
  featureVersion: number;
  /** Local test id -> stored test case, so a save updates rather than duplicates. */
  tests: Record<string, { id: string; version: number; hash: string }>;
  /** Fingerprint of the stored graph, so an unchanged one is not sent again. */
  modelHash?: string;
  /** Fingerprint of everything stored, so a no-op save is skipped. */
  lastHash?: string;
  scenario: string;
}

/** Ids of the stored copy, for callers that need them (the AI panel). */
export interface RemoteIds {
  featureId: string;
  modelId: string;
}

@Injectable()
export class ModelPersistenceService {
  private readonly api = inject(OrgApi);

  readonly state = signal<SaveState>('idle');
  readonly savedAt = signal<Date | null>(null);
  readonly error = signal<string | null>(null);
  /** The feature the open model belongs to, once loaded. */
  readonly featureId = signal<string | null>(null);
  readonly featureName = signal<string | null>(null);

  /** True while the database copy may be behind what is on screen. */
  readonly unsaved = computed(() =>
    ['pending', 'saving', 'offline', 'error', 'conflict'].includes(this.state()));

  private modelId: string | null = null;
  private latest: PersistedModel | null = null;
  private rec: Remote | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private running: Promise<RemoteIds | null> | null = null;
  private again = false;

  constructor() {
    inject(DestroyRef).onDestroy(() => {
      // Leaving the editor: save what is pending rather than waiting.
      if (this.timer !== null) {
        clearTimeout(this.timer);
        this.timer = null;
        void this.flush();
      }
    });
  }

  /** Fetches the stored model; resolves to `null` when it cannot be loaded. */
  async open(modelId: string): Promise<PersistedModel | null> {
    this.cancelTimer();
    this.modelId = modelId;
    this.latest = null;
    this.rec = null;
    this.error.set(null);
    this.savedAt.set(null);
    this.state.set('loading');

    try {
      const model = await this.api.getModel(modelId);
      const [testCases, feature] = await Promise.all([
        this.api.modelTestCases(modelId),
        this.api.getFeature(model.featureId),
      ]);
      if (this.modelId !== modelId) return null;   // another model was opened meanwhile

      const merged = fromRemote(
        model, testCases, modelId, feature.scenarioDescription ?? '', null);
      merged.id = modelId;

      const tests: Remote['tests'] = {};
      for (const node of merged.nodes) {
        for (const t of node.tests) {
          const tc = testCases.find(c => c.id === t.id);
          if (tc) {
            tests[t.id] = {
              id: tc.id, version: tc.version, hash: hashOf(testToInput(t, modelId, node.id)),
            };
          }
        }
      }
      this.rec = {
        version: model.version,
        featureId: feature.id,
        featureVersion: feature.version,
        tests,
        modelHash: hashOf(toModelInput(merged)),
        lastHash: remoteFingerprint(merged),
        scenario: merged.scenarioDesc,
      };
      this.latest = merged;
      this.featureId.set(feature.id);
      this.featureName.set(feature.name);
      this.state.set('saved');
      this.savedAt.set(new Date(model.updatedAt));
      return merged;
    } catch (e) {
      this.fail(e, 'The model could not be loaded.');
      return null;
    }
  }

  /**
   * Notes a change. The save starts `SAVE_DEBOUNCE_MS` after the last one, so
   * a burst of edits — dragging a state, typing a name — is one request.
   */
  schedule(model: PersistedModel): void {
    if (!this.rec || model.id !== this.modelId) return;
    this.latest = model;
    if (this.rec.lastHash === remoteFingerprint(model)) {
      // Only presentation changed (a colour, a curve): nothing to send.
      if (this.state() === 'pending') this.state.set('saved');
      return;
    }
    if (this.state() === 'conflict') return;
    this.state.set('pending');
    this.arm(SAVE_DEBOUNCE_MS);
  }

  /** Saves now, and resolves to the stored ids (or null if it could not). */
  async flush(): Promise<RemoteIds | null> {
    this.cancelTimer();
    if (this.running) {
      // A save is in flight; run once more after it, with the newest content.
      this.again = true;
      return this.running;
    }
    this.running = this.save().finally(() => { this.running = null; });
    const result = await this.running;
    if (this.again) {
      this.again = false;
      return this.flush();
    }
    return result;
  }

  /** After a conflict: write this version over the other one. */
  async keepMine(): Promise<void> {
    if (!this.modelId || !this.rec) return;
    try {
      const current = await this.api.getModel(this.modelId);
      this.rec = { ...this.rec, version: current.version, modelHash: undefined, lastHash: undefined };
    } catch (e) {
      this.fail(e, 'The model could not be saved.');
      return;
    }
    this.state.set('pending');
    await this.flush();
  }

  /** After a conflict: take the other version. Resolves to it, for the editor. */
  async takeTheirs(): Promise<PersistedModel | null> {
    return this.modelId ? this.open(this.modelId) : null;
  }

  /** Deletes the stored copy of the open model. */
  async remove(): Promise<void> {
    const id = this.modelId;
    this.cancelTimer();
    this.modelId = null;
    this.latest = null;
    this.rec = null;
    if (!id) return;
    try {
      await this.api.deleteModel(id);
    } catch (e) {
      if (!(e instanceof ApiError && e.status === 404)) throw e;
    }
  }

  // ── Saving ────────────────────────────────────────────────────────────────

  private async save(): Promise<RemoteIds | null> {
    const modelId = this.modelId;
    const model = this.latest;
    let rec = this.rec;
    if (!modelId || !model || !rec) return null;
    if (this.state() === 'conflict') return null;

    this.state.set('saving');
    try {
      const hash = remoteFingerprint(model);

      const input = toModelInput(model);
      const modelHash = hashOf(input);
      if (rec.modelHash !== modelHash) {
        const saved = await this.api.replaceModel(modelId, rec.version, input);
        rec = { ...rec, version: saved.version, modelHash };
        this.rec = rec;
      }

      if (rec.scenario !== (model.scenarioDesc ?? '')) {
        rec = await this.saveScenario(rec, model.scenarioDesc ?? '');
        this.rec = rec;
      }

      // Test cases, after the states they are assigned to exist.
      rec = await this.saveTests(model, rec, modelId);

      this.rec = { ...rec, lastHash: hash };
      this.state.set('saved');
      this.savedAt.set(new Date());
      this.error.set(null);
      return { featureId: rec.featureId, modelId };
    } catch (e) {
      this.fail(e, 'The model could not be saved.');
      return null;
    }
  }

  /** The scenario is one text field and the user is looking at their version, so a 412 is retried. */
  private async saveScenario(rec: Remote, text: string): Promise<Remote> {
    const patch = { name: this.featureName() || 'Feature', scenarioDescription: text };
    try {
      const saved = await this.api.updateFeature(rec.featureId, rec.featureVersion, patch);
      return { ...rec, featureVersion: saved.version, scenario: text };
    } catch (e) {
      if (!(e instanceof ApiError && e.status === 412)) throw e;
      const current = await this.api.getFeature(rec.featureId);
      const saved = await this.api.updateFeature(current.id, current.version, patch);
      return { ...rec, featureVersion: saved.version, scenario: text };
    }
  }

  private async saveTests(model: PersistedModel, rec: Remote, modelId: string): Promise<Remote> {
    const tests = { ...rec.tests };
    const seen = new Set<string>();
    for (const node of model.nodes) {
      for (const t of node.tests) {
        seen.add(t.id);
        const input = testToInput(t, modelId, node.id);
        const hash = hashOf(input);
        const known = tests[t.id];
        if (known && known.hash === hash) continue;
        if (known) {
          try {
            const saved = await this.api.replaceTestCase(known.id, known.version, input);
            tests[t.id] = { id: saved.id, version: saved.version, hash };
            continue;
          } catch (e) {
            // Deleted elsewhere: store it again. Anything else, a 412 included,
            // is the caller's to report.
            if (!(e instanceof ApiError && e.status === 404)) throw e;
          }
        }
        const created = await this.api.createTestCase(rec.featureId, input);
        tests[t.id] = { id: created.id, version: created.version, hash };
      }
    }

    // Removed in the editor. Only test cases this editor loaded or created are
    // deleted: one may be assigned in other models too, and those are not ours.
    for (const [localTestId, stored] of Object.entries(tests)) {
      if (seen.has(localTestId)) continue;
      try {
        await this.api.deleteTestCase(stored.id);
      } catch (e) {
        if (!(e instanceof ApiError && e.status === 404)) throw e;
      }
      delete tests[localTestId];
    }
    return { ...rec, tests };
  }

  // ── Helpers ───────────────────────────────────────────────────────────────

  private fail(e: unknown, fallback: string): void {
    if (e instanceof ApiError && e.status === 412) {
      this.state.set('conflict');
      this.error.set('Someone else changed this model since it was opened.');
      return;
    }
    if (e instanceof ApiError && e.status === 0) {
      this.state.set('offline');
      this.error.set('The server cannot be reached. Changes are retried.');
      if (this.rec) this.arm(OFFLINE_RETRY_MS);
      return;
    }
    this.state.set('error');
    this.error.set(e instanceof ApiError ? e.message : fallback);
  }

  private arm(ms: number): void {
    this.cancelTimer();
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.flush();
    }, ms);
  }

  private cancelTimer(): void {
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }
}

function hashOf(value: unknown): string {
  return JSON.stringify(value);
}
