import {
  ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked,
} from '@angular/core';
import { NgTemplateOutlet } from '@angular/common';
import { RouterLink } from '@angular/router';
import { ApiError } from '../../../core/api/api-error';
import {
  Traceability, TraceabilityApi, TraceGap, TraceItem, TraceTestCase,
} from '../../../core/api/traceability-api';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { linkLabel, safeExternalUrl } from '../../models/state/model-editor.store';
import {
  coversNoElement, searchTrace, toCsv, toRows, TraceNames,
} from '../state/traceability-view';

type View = 'matrix' | 'gaps';

/** What each gap means, for the gap filter, the gaps tab and the row flags. */
const GAPS: { value: TraceGap; label: string; hint: string }[] = [
  { value: 'untraced',      label: 'No backlog item',
    hint: 'Test cases that link no backlog item' },
  { value: 'unimplemented', label: 'No implementation',
    hint: 'Test cases of a backlog item that have no implementation link' },
  { value: 'no-elements',   label: 'No model element',
    hint: 'Backlog items none of whose test cases is assigned to a state or transition' },
];

const EMPTY: Traceability = {
  items: [], untraced: [],
  summary: { backlogItems: 0, testCases: 0, untraced: 0, unimplemented: 0, noElements: 0 },
};

@Component({
  selector: 'tm-traceability-page',
  imports: [NgTemplateOutlet, RouterLink],
  templateUrl: './traceability-page.html',
  styleUrl: './traceability-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TraceabilityPageComponent {
  private readonly explorer = inject(ExplorerStore);
  private readonly api = inject(TraceabilityApi);

  readonly gaps = GAPS;
  readonly projects = this.explorer.projects;

  readonly view = signal<View>('matrix');
  readonly projectId = signal<string | null>(null);
  readonly componentId = signal('');
  readonly featureId = signal('');
  readonly gap = signal<TraceGap | null>(null);
  readonly search = signal('');

  readonly trace = signal<Traceability>(EMPTY);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);
  private requests = 0;

  readonly components = computed(() =>
    this.projects().find(p => p.id === this.projectId())?.components ?? []);
  readonly features = computed(() =>
    this.components().find(c => c.id === this.componentId())?.features ?? []);

  private readonly componentNames = computed(() => new Map(
    this.projects().flatMap(p => p.components).map(c => [c.id, c.name] as const)));
  private readonly featureNames = computed(() => new Map(
    this.projects().flatMap(p => p.components).flatMap(c => c.features)
      .map(f => [f.id, f.name] as const)));

  /** Component and feature names for the trace's ids; an unknown id shows as itself. */
  readonly names = computed<TraceNames>(() => {
    const components = this.componentNames();
    const features = this.featureNames();
    return {
      component: id => components.get(id) ?? id,
      feature: id => features.get(id) ?? id,
    };
  });

  /** The trace with the search applied (the gap is applied by the server). */
  readonly shown = computed(() =>
    searchTrace(this.trace().items, this.trace().untraced, this.search()));
  readonly rows = computed(() => toRows(this.shown().items, this.shown().untraced));
  readonly summary = computed(() => this.trace().summary);

  constructor() {
    // Open on the first project, and move off one that was deleted.
    effect(() => {
      const projects = this.projects();
      const current = untracked(this.projectId);
      if (!projects.some(p => p.id === current)) this.projectId.set(projects[0]?.id ?? null);
    });
    effect(() => {
      const query = {
        projectId: this.projectId(),
        componentId: this.componentId() || undefined,
        featureId: this.featureId() || undefined,
        gap: this.gap() ?? undefined,
      };
      untracked(() => void this.load(query));
    });
  }

  private async load(q: {
    projectId: string | null; componentId?: string; featureId?: string; gap?: TraceGap;
  }): Promise<void> {
    if (!q.projectId) {
      this.trace.set(EMPTY);
      return;
    }
    const request = ++this.requests;
    this.loading.set(true);
    try {
      const trace = await this.api.get(q.projectId, q);
      if (request !== this.requests) return;
      this.trace.set(trace);
      this.error.set(null);
    } catch (e) {
      if (request !== this.requests) return;
      this.trace.set(EMPTY);
      this.error.set(e instanceof ApiError ? e.message : 'The traceability could not be loaded.');
    } finally {
      if (request === this.requests) this.loading.set(false);
    }
  }

  // ── Filters ───────────────────────────────────────────────────────────────
  setProject(id: string): void {
    this.projectId.set(id);
    this.componentId.set('');
    this.featureId.set('');
  }

  setComponent(id: string): void {
    this.componentId.set(id);
    this.featureId.set('');
  }

  setGap(value: string): void {
    this.gap.set(GAPS.some(g => g.value === value) ? (value as TraceGap) : null);
  }

  setView(view: View): void {
    this.view.set(view);
    // The gaps tab always shows one gap; the matrix shows whatever was chosen.
    if (view === 'gaps' && !this.gap()) this.gap.set('untraced');
  }

  gapCount(gap: TraceGap): number {
    const s = this.summary();
    return gap === 'untraced' ? s.untraced : gap === 'unimplemented' ? s.unimplemented : s.noElements;
  }

  gapLabel(gap: TraceGap | null): string {
    return GAPS.find(g => g.value === gap)?.label ?? '';
  }

  // ── Row helpers ───────────────────────────────────────────────────────────
  href(url: string | undefined): string | null { return safeExternalUrl(url); }
  label(url: string | undefined): string { return linkLabel(url) || (url ?? ''); }
  noElement(item: TraceItem): boolean { return coversNoElement(item); }
  unimplemented(tc: TraceTestCase): boolean { return !tc.implementationUrl; }

  // ── Export ────────────────────────────────────────────────────────────────
  exportCsv(): void {
    const rows = this.rows();
    if (!rows.length) return;
    const url = URL.createObjectURL(
      new Blob([toCsv(rows, this.names())], { type: 'text/csv;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = 'traceability.csv';
    a.click();
    URL.revokeObjectURL(url);
  }
}
