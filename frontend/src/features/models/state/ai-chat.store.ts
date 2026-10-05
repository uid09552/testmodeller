/**
 * State of the AI chat panel in the model editor.
 *
 * Provided at model-editor-page level, next to `ModelEditorStore`, so the
 * transcript survives switching the right panel between Properties and the
 * chat. One user message is one proposal request — see ADR 0003.
 */
import { computed, inject, Injectable, signal } from '@angular/core';
import { AiApi } from '../../../core/api/ai-api';
import { ApiError } from '../../../core/api/api-error';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import {
  ElementsPayload, FeatureDescriptionPayload, ModelInput, Proposal,
  ProposalKind, TestCaseInput,
} from '../../../core/api/api.types';
import {
  ChatIntent, GraphDraft, inferKind, kindLabel, testCaseToDraft, toGraphDraft,
} from './ai-chat.mapping';
import { ModelEditorStore, NODE_H, NODE_W } from './model-editor.store';
import { ModelPersistenceService } from './model-persistence';

/** Something the user or the agent said. */
export interface ChatSaid {
  kind: 'said';
  id: string;
  role: 'user' | 'agent';
  text: string;
  at: Date;
}

/** A failed turn, with the API's own explanation. */
export interface ChatFailure {
  kind: 'failure';
  id: string;
  text: string;
  /** What the user can do about it, when that is knowable. */
  hint?: string;
  at: Date;
}

/** One proposal, awaiting the user's decision (FR-033). */
export interface ChatCard {
  kind: 'card';
  id: string;
  proposal: Proposal;
  state: 'pending' | 'working' | 'accepted' | 'rejected';
  /** What applying it did to the model, once accepted. */
  applied?: string;
  error?: string;
  at: Date;
}

export type ChatEntry = ChatSaid | ChatFailure | ChatCard;

/**
 * Which model the chat is about, in the editor's own terms.
 *
 * The backend ids come from `ModelPersistenceService`, which stores the model
 * (ids in the editor are local) before the AI endpoints are given a feature.
 */
export interface ChatContext {
  /** The id the editor is working with, as used by the tree. */
  localModelId: string | null;
  modelName: string | null;
}

/** Horizontal/vertical spacing used when a proposal brings no positions. */
const GRID_X = NODE_W + 80;
const GRID_Y = NODE_H + 90;
const GRID_COLS = 3;

@Injectable()
export class AiChatStore {
  private readonly api = inject(AiApi);
  private readonly persistence = inject(ModelPersistenceService);
  private readonly explorer = inject(ExplorerStore);
  private readonly model = inject(ModelEditorStore);

  readonly entries = signal<ChatEntry[]>([]);

  /** Non-null while a turn is in flight; the text is shown as a status line. */
  readonly working = signal<string | null>(null);

  /** What the composer will ask for. */
  readonly intent = signal<ChatIntent>('auto');

  readonly context = signal<ChatContext>({ localModelId: null, modelName: null });

  /** Where the open model sits in the tree, which is what gets synced. */
  readonly localPath = computed(() => {
    const id = this.context().localModelId;
    if (!id) return null;
    return this.explorer.modelPaths().find(p => p.model.id === id) ?? null;
  });

  /** The chat can run once the open model belongs to a feature in the tree. */
  readonly ready = computed(() => this.localPath() !== null);

  readonly pendingCount = computed(() =>
    this.entries().filter(e => e.kind === 'card' && e.state === 'pending').length);

  private abort: AbortController | null = null;

  setContext(ctx: ChatContext): void { this.context.set(ctx); }

  /** Which kind the current composer state would send for `text`. */
  resolvedKind(text: string): ProposalKind {
    const intent = this.intent();
    return intent === 'auto' ? inferKind(text) : intent;
  }

  /**
   * Sends one message: request proposals, wait for the job, then show each
   * proposal as a card. Nothing touches the model until the user accepts.
   */
  async send(message: string): Promise<void> {
    const text = message.trim();
    if (!text || this.working()) return;

    this.push({ kind: 'said', id: uid(), role: 'user', text, at: new Date() });

    const path = this.localPath();
    if (!path) {
      this.fail(
        'This model is not in a project yet, so there is no feature to ask about.',
        'Create it from the Projects tree, or open it from there.',
      );
      return;
    }

    const kind = this.resolvedKind(text);
    this.abort = new AbortController();
    try {
      // Store the model first so the backend asks about what is on screen.
      this.working.set('Saving this model…');
      const ctx = await this.persistence.flush();
      if (!ctx) {
        this.fail(
          this.persistence.error() ?? 'The model could not be saved.',
          'The AI works from the stored model, so it has to be saved first.',
        );
        return;
      }

      this.working.set(`Asking for ${kindLabel(kind).toLowerCase()}…`);
      const job = await this.api.requestProposals({
        kind,
        featureId: ctx.featureId,
        // Only sent where it means something; the contract requires it for
        // states-and-transitions and ignores it elsewhere.
        modelId: ctx.modelId,
        prompt: text,
      });
      this.working.set('Waiting for the model…');
      const finished = await this.api.awaitJob(job.id, this.abort.signal);
      if (finished.status === 'failed') {
        this.fail(
          finished.error?.detail ?? finished.error?.title ?? 'The AI job failed.',
          'The server log has the provider error; check Settings if no provider is configured.',
        );
        return;
      }
      const ids = finished.proposalIds ?? [];
      if (ids.length === 0) {
        this.say('agent', 'The job finished without producing any proposal.');
        return;
      }
      this.working.set(`Loading ${ids.length} proposal(s)…`);
      const proposals = await Promise.all(ids.map(id => this.api.getProposal(id)));
      this.say(
        'agent',
        `Here ${proposals.length === 1 ? 'is 1 proposal' : `are ${proposals.length} proposals`} `
        + `for ${kindLabel(kind).toLowerCase()}. Nothing is applied until you accept.`,
      );
      for (const proposal of proposals) {
        this.push({
          kind: 'card', id: uid(), proposal, state: 'pending', at: new Date(),
        });
      }
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') {
        this.say('agent', 'Cancelled. The job may still finish on the server.');
      } else {
        this.reportError(e);
      }
    } finally {
      this.working.set(null);
      this.abort = null;
    }
  }

  /** Stops waiting for the current job. */
  cancel(): void { this.abort?.abort(); }

  clear(): void { this.entries.set([]); }

  /**
   * Accepts a proposal: the API records it and creates the entity, and only
   * then is it applied to the open model (FR-033, AGENTS.md rule 8).
   */
  async accept(card: ChatCard): Promise<void> {
    if (card.state !== 'pending') return;
    this.patchCard(card.id, { state: 'working', error: undefined });
    try {
      const accepted = await this.api.acceptProposal(card.proposal.id);
      const applied = this.apply(accepted);
      this.patchCard(card.id, { state: 'accepted', proposal: accepted, applied });
    } catch (e) {
      const err = e instanceof ApiError ? e.message : 'Could not accept the proposal.';
      this.patchCard(card.id, { state: 'pending', error: err });
    }
  }

  async reject(card: ChatCard): Promise<void> {
    if (card.state !== 'pending') return;
    this.patchCard(card.id, { state: 'working', error: undefined });
    try {
      const rejected = await this.api.rejectProposal(card.proposal.id);
      this.patchCard(card.id, { state: 'rejected', proposal: rejected });
    } catch (e) {
      const err = e instanceof ApiError ? e.message : 'Could not reject the proposal.';
      this.patchCard(card.id, { state: 'pending', error: err });
    }
  }

  // ── Applying an accepted proposal to the open model ────────────────────────

  /** Returns a one-line summary of what was applied. */
  private apply(proposal: Proposal): string {
    switch (proposal.kind) {
      case 'test-cases':
        return this.applyTestCase(proposal.payload as TestCaseInput);
      case 'feature-description':
        return this.applyScenario(proposal.payload as FeatureDescriptionPayload);
      case 'states-and-transitions':
        return this.applyGraph(toGraphDraft(proposal.payload as ElementsPayload));
      case 'model':
        return this.applyGraph(toGraphDraft(proposal.payload as ModelInput));
    }
  }

  /**
   * Attaches the test case to a state. The canvas assigns test cases to
   * states, so a target is needed: the selection, else the initial state, else
   * the first one.
   */
  private applyTestCase(input: TestCaseInput): string {
    const target =
      this.model.selectedNode()
      ?? this.model.nodes().find(n => n.kind === 'initial')
      ?? this.model.nodes()[0];
    if (!target) {
      return 'Accepted. The model has no state to attach it to yet, '
        + 'so it exists on the server only.';
    }
    const draft = testCaseToDraft(input);
    const test = this.model.addTest(target.id, draft.name);
    if (!test) return 'Accepted on the server, but the target state disappeared.';
    this.model.updateTest(target.id, test.id, {
      category: draft.category,
      polarity: draft.polarity,
      given: draft.given || `the system is in the "${target.label}" state`,
      when: draft.when,
      then: draft.then,
    });
    this.model.select(target.id, 'node');
    this.model.focusTests();
    return `Added to "${target.label}" as ${this.model.testRef(test)}.`;
  }

  private applyScenario(payload: FeatureDescriptionPayload): string {
    const text = payload.scenarioDescription?.trim() ?? '';
    if (!text) return 'Accepted, but the proposal carried no description.';
    this.model.setScenarioDesc(text);
    return 'Replaced the scenario description.';
  }

  private applyGraph(draft: GraphDraft): string {
    if (draft.states.length === 0 && draft.transitions.length === 0) {
      return 'Accepted, but the proposal carried no states or transitions.';
    }
    this.model.checkpoint();
    // New elements start below everything already on the canvas, so an
    // accepted proposal never lands on top of existing work.
    const baseY = this.model.nodes()
      .reduce((max, n) => Math.max(max, n.y + n.h), 0) + (this.model.nodes().length ? GRID_Y : 60);

    const ids = new Map<string, string>();
    draft.states.forEach((s, i) => {
      const x = s.position?.x ?? 80 + (i % GRID_COLS) * GRID_X;
      const y = s.position?.y ?? baseY + Math.floor(i / GRID_COLS) * GRID_Y;
      const node = this.model.addNode(s.kind, x, y);
      this.model.updateNode(node.id, { label: s.name, description: s.description });
      ids.set(s.key, node.id);
    });

    let edges = 0;
    for (const t of draft.transitions) {
      const from = ids.get(t.from);
      const to = ids.get(t.to);
      if (!from || !to) continue;
      const edge = this.model.addEdge(from, to);
      this.model.updateEdge(edge.id, {
        label: t.event, guard: t.guard, action: t.action,
      });
      edges += 1;
    }
    return `Added ${draft.states.length} state(s) and ${edges} transition(s).`;
  }

  // ── Transcript helpers ────────────────────────────────────────────────────
  private push(entry: ChatEntry): void {
    this.entries.update(es => [...es, entry]);
  }

  private say(role: 'user' | 'agent', text: string): void {
    this.push({ kind: 'said', id: uid(), role, text, at: new Date() });
  }

  private fail(text: string, hint?: string): void {
    this.push({ kind: 'failure', id: uid(), text, hint, at: new Date() });
  }

  private reportError(e: unknown): void {
    if (!(e instanceof ApiError)) {
      this.fail(e instanceof Error ? e.message : 'Unexpected error.');
      return;
    }
    this.fail(e.message, hintFor(e));
  }

  private patchCard(id: string, patch: Partial<Omit<ChatCard, 'kind' | 'id'>>): void {
    this.entries.update(es => es.map(e =>
      e.kind === 'card' && e.id === id ? { ...e, ...patch } : e));
  }
}

/** What the user can do about a given failure. */
function hintFor(e: ApiError): string | undefined {
  switch (e.status) {
    case 0:
      return 'Start the backend (make dev) and reload.';
    case 401:
    case 403:
      return 'Sign in again — the gateway did not accept the request.';
    case 404:
      return 'The backend lost the synced copy. Ask again — it is recreated.';
    case 503:
      return 'No AI provider is configured. Set one up in Settings.';
    default:
      return undefined;
  }
}

function uid(): string { return crypto.randomUUID(); }
