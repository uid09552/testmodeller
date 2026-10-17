import { computed, inject, Injectable, signal } from '@angular/core';
import { ApiError } from '../../../core/api/api-error';
import { ThreatModelIndex } from './threat-model-index';
import {
  ArchComponent, Asset, CollectionName, Collections, Control, Flow, SecurityTest, Threat,
  ThreatModel, ThreatModelApi, ThreatModelInput, ThreatModelSummary, Zone,
} from '../../../core/api/threat-model-api';

/** What a reader is told when someone else saved first (412). */
export const CONFLICT_MESSAGE = 'Someone else changed this in the meantime. Reload and try again.';

/** A user-facing message for a failed request. */
export function errorMessage(e: unknown): string {
  if (e instanceof ApiError && e.status === 412) return CONFLICT_MESSAGE;
  return e instanceof Error ? e.message : String(e);
}

/**
 * State of one project's threat model, provided by the workspace page.
 *
 * Every write is followed by a reload of the threat model: one change can alter
 * computed fields elsewhere (a zone move changes crossings, a control link
 * changes a threat's flags), and the server is the only place that computes
 * them (ADR 0011).
 */
@Injectable()
export class ThreatModelStore {
  private readonly api = inject(ThreatModelApi);
  private readonly index = inject(ThreatModelIndex);

  readonly projectId = signal<string | null>(null);
  readonly model = signal<ThreatModel | null>(null);
  /** The project has no threat model yet. */
  readonly missing = signal(false);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  readonly zones = signal<Zone[]>([]);
  readonly components = signal<ArchComponent[]>([]);
  readonly assets = signal<Asset[]>([]);
  readonly flows = signal<Flow[]>([]);
  readonly threats = signal<Threat[]>([]);
  readonly controls = signal<Control[]>([]);
  readonly tests = signal<SecurityTest[]>([]);
  readonly summary = signal<ThreatModelSummary | null>(null);

  readonly id = computed(() => this.model()?.id ?? null);
  private generation = 0;

  /** Loads the threat model of a project; none is created. */
  async load(projectId: string): Promise<void> {
    const gen = ++this.generation;
    this.projectId.set(projectId);
    this.loading.set(true);
    this.error.set(null);
    try {
      const model = await this.api.ofProject(projectId);
      if (gen !== this.generation) return;
      this.model.set(model);
      this.missing.set(false);
      this.index.set(projectId, { id: model.id, name: model.name });
      await this.refresh(gen);
    } catch (e) {
      if (gen !== this.generation) return;
      this.clear();
      if (e instanceof ApiError && e.status === 404) {
        this.missing.set(true);
        this.index.set(projectId, null);
      } else {
        this.error.set(errorMessage(e));
      }
    } finally {
      if (gen === this.generation) this.loading.set(false);
    }
  }

  private clear(): void {
    this.model.set(null);
    this.zones.set([]);
    this.components.set([]);
    this.assets.set([]);
    this.flows.set([]);
    this.threats.set([]);
    this.controls.set([]);
    this.tests.set([]);
    this.summary.set(null);
  }

  /** Reloads every collection and the summary. */
  async refresh(gen = this.generation): Promise<void> {
    const id = this.id();
    if (!id) return;
    const [model, zones, components, assets, flows, threats, controls, tests, summary] = await Promise.all([
      this.api.ofProject(this.projectId()!),
      this.api.list(id, 'zones'),
      this.api.list(id, 'components'),
      this.api.list(id, 'assets'),
      this.api.list(id, 'flows'),
      this.api.list(id, 'threats'),
      this.api.list(id, 'controls'),
      this.api.list(id, 'security-tests'),
      this.api.summary(id),
    ]);
    if (gen !== this.generation) return;
    this.model.set(model);
    this.index.set(this.projectId()!, { id: model.id, name: model.name });
    this.zones.set(zones);
    this.components.set(components);
    this.assets.set(assets);
    this.flows.set(flows);
    this.threats.set(threats);
    this.controls.set(controls);
    this.tests.set(tests);
    this.summary.set(summary);
  }

  /** Runs a write, reloads, and reports failure in `error` as well as by rethrowing. */
  private async write<T>(op: (id: string) => Promise<T>): Promise<T> {
    const id = this.id();
    if (!id) throw new Error('No threat model is loaded');
    this.error.set(null);
    try {
      const result = await op(id);
      await this.refresh();
      return result;
    } catch (e) {
      this.error.set(errorMessage(e));
      throw e;
    }
  }

  /** Creates the project's threat model. */
  async create(name: string): Promise<void> {
    const projectId = this.projectId();
    if (!projectId) return;
    this.error.set(null);
    try {
      const created = await this.api.create(projectId, { name });
      this.model.set(created);
      this.index.set(projectId, { id: created.id, name: created.name });
      this.missing.set(false);
      await this.refresh();
    } catch (e) {
      this.error.set(errorMessage(e));
    }
  }

  saveModel(input: ThreatModelInput): Promise<ThreatModel> {
    return this.write(id => this.api.update(id, this.model()!.version, input));
  }

  /** Creates or replaces an item; `existing` decides which. */
  save<C extends CollectionName>(
    c: C, input: Collections[C]['input'], existing?: { id: string; version: number },
  ): Promise<Collections[C]['item']> {
    return this.write(id => existing
      ? this.api.updateItem(id, c, existing.id, existing.version, input)
      : this.api.createItem(id, c, input));
  }

  remove(c: CollectionName, itemId: string): Promise<void> {
    return this.write(id => this.api.deleteItem(id, c, itemId));
  }

  suggestThreats() {
    return this.write(id => this.api.suggestThreats(id));
  }

  bulkThreatStatus(ids: string[], status: 'open' | 'dismissed', reason?: string) {
    return this.write(id => this.api.bulkThreatStatus(id, ids, status, reason));
  }

  generateTests(threatIds?: string[]) {
    return this.write(id => this.api.generateTests(id, threatIds));
  }

  bulkTestStatus(ids: string[], status: SecurityTest['status']) {
    return this.write(id => this.api.bulkTestStatus(id, ids, status));
  }

  recordResult(testId: string, outcome: 'passed' | 'failed' | 'not-run', note?: string, executedAt?: string) {
    return this.write(id => this.api.recordResult(id, testId, { outcome, note, executedAt }));
  }

  /** Saves the diagram layout; a newer layout from elsewhere wins (412 → reload). */
  async saveLayout(layout: Record<string, unknown>): Promise<void> {
    const m = this.model();
    if (!m) return;
    try {
      const { layoutVersion } = await this.api.saveLayout(m.id, m.layoutVersion, layout);
      this.model.update(x => x ? { ...x, layout, layoutVersion } : x);
    } catch (e) {
      this.error.set(errorMessage(e));
      if (e instanceof ApiError && e.status === 412) await this.refresh();
    }
  }
}
