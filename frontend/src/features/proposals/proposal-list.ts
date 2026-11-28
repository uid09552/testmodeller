import {
  ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked,
} from '@angular/core';
import {
  IxButton, IxCard, IxCardContent, IxEmptyState, IxMessageBar, IxPill, IxSelect, IxSelectItem,
  IxSpinner, ToastService,
} from '@siemens/ix-angular/standalone';
import { AiApi } from '../../core/api/ai-api';
import { ApiError } from '../../core/api/api-error';
import { Proposal, ProposalStatus } from '../../core/api/api.types';
import { ExplorerStore } from '../explorer/state/explorer.store';
import { details, KIND_LABEL, summary } from './proposal-view';

type Filter = 'all' | ProposalStatus;

const FILTERS: { value: Filter; label: string }[] = [
  { value: 'pending', label: 'Pending' },
  { value: 'accepted', label: 'Accepted' },
  { value: 'rejected', label: 'Rejected' },
  { value: 'all', label: 'All' },
];

/**
 * AI proposals of a project (FR-030..FR-034). A proposal is not accepted data
 * until a person accepts it here (AGENTS.md rule 8; DESIGN.md "AI proposals").
 */
@Component({
  selector: 'tm-proposal-list',
  imports: [
    IxButton, IxCard, IxCardContent, IxEmptyState, IxMessageBar, IxPill, IxSelect, IxSelectItem,
    IxSpinner,
  ],
  templateUrl: './proposal-list.html',
  styleUrl: './proposal-list.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProposalListComponent {
  private readonly api = inject(AiApi);
  private readonly explorer = inject(ExplorerStore);
  private readonly toasts = inject(ToastService);

  readonly projects = this.explorer.projects;
  readonly projectId = signal<string | null>(null);
  readonly filter = signal<Filter>('pending');
  readonly filters = FILTERS;
  readonly proposals = signal<Proposal[]>([]);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);
  /** The proposal being accepted or rejected, so its buttons show progress. */
  readonly busyId = signal<string | null>(null);
  private requests = 0;

  readonly kindLabel = KIND_LABEL;
  readonly summary = summary;
  readonly details = details;

  readonly pendingCount = computed(() => this.proposals().filter(p => p.status === 'pending').length);
  readonly shown = computed(() => {
    const f = this.filter();
    return this.proposals().filter(p => f === 'all' || p.status === f);
  });

  /** "Checkout › Cart › Login flow" for a proposal's feature and model, from the tree. */
  readonly targetOf = computed(() => {
    const paths = this.explorer.modelPaths();
    const features = new Map(this.projects().flatMap(p => p.components.flatMap(c =>
      c.features.map(f => [f.id, `${c.name} › ${f.name}`] as const))));
    return (p: Proposal): string => {
      const model = p.modelId ? paths.find(m => m.model.id === p.modelId)?.model.name : undefined;
      const feature = p.featureId ? features.get(p.featureId) : undefined;
      return [feature, model].filter(Boolean).join(' › ') || 'Project';
    };
  });

  constructor() {
    effect(() => {
      const projects = this.projects();
      const current = untracked(this.projectId);
      if (!projects.some(p => p.id === current)) this.projectId.set(projects[0]?.id ?? null);
    });
    effect(() => {
      const id = this.projectId();
      untracked(() => void this.load(id));
    });
  }

  setProject(e: Event | string): void {
    const raw = typeof e === 'string' ? e : (e as CustomEvent<string | string[]>).detail;
    const id = Array.isArray(raw) ? raw[0] : raw;
    if (id && this.projects().some(p => p.id === id)) this.projectId.set(id);
  }

  setFilter(e: Event | Filter): void {
    const raw = typeof e === 'string' ? e : (e as CustomEvent<string | string[]>).detail;
    const v = (Array.isArray(raw) ? raw[0] : raw) as Filter;
    if (FILTERS.some(f => f.value === v)) this.filter.set(v);
  }

  private async load(projectId: string | null): Promise<void> {
    const request = ++this.requests;
    if (!projectId) {
      this.proposals.set([]);
      return;
    }
    this.loading.set(true);
    try {
      const list = await this.api.listProposals(projectId);
      if (request !== this.requests) return;
      this.proposals.set(list);
      this.error.set(null);
    } catch (e) {
      if (request !== this.requests) return;
      this.error.set(e instanceof ApiError ? e.message : 'The proposals could not be loaded.');
    } finally {
      if (request === this.requests) this.loading.set(false);
    }
  }

  /** The only way a proposal becomes accepted data (AGENTS.md rule 8). */
  accept(p: Proposal): Promise<void> {
    return this.decide(p, () => this.api.acceptProposal(p.id), 'Accepted: the change is now in the project.');
  }

  reject(p: Proposal): Promise<void> {
    return this.decide(p, () => this.api.rejectProposal(p.id), 'Rejected: nothing was changed.');
  }

  private async decide(p: Proposal, call: () => Promise<Proposal>, done: string): Promise<void> {
    this.busyId.set(p.id);
    try {
      const updated = await call();
      this.proposals.update(ps => ps.map(x => x.id === updated.id ? updated : x));
      void this.toasts.show({ type: 'success', message: done });
    } catch (e) {
      void this.toasts.show({
        type: 'error',
        message: e instanceof ApiError ? e.message : 'The decision could not be saved; the proposal is still pending.',
      });
    } finally {
      this.busyId.set(null);
    }
  }

  statusVariant(s: ProposalStatus): 'information' | 'success' | 'neutral' {
    return s === 'pending' ? 'information' : s === 'accepted' ? 'success' : 'neutral';
  }

  statusLabel(s: ProposalStatus): string {
    return s === 'pending' ? 'Proposal' : s === 'accepted' ? '✓ Accepted' : '✗ Rejected';
  }
}
