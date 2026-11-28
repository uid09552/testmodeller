import {
  ChangeDetectionStrategy, Component, computed, effect, inject, signal, untracked,
} from '@angular/core';
import { RouterLink } from '@angular/router';
import {
  IxEmptyState, IxKpi, IxMessageBar, IxPill, IxProgressIndicator, IxSelect, IxSelectItem,
  IxSpinner,
} from '@siemens/ix-angular/standalone';
import { ApiError } from '../../core/api/api-error';
import { OrgApi } from '../../core/api/org-api';
import { ExplorerStore } from '../explorer/state/explorer.store';
import {
  byFeature, Count, level, LEVEL_LABEL, LEVEL_VARIANT, ModelRow, modelRows, percent, totals,
  withCoverage,
} from './coverage-view';

/**
 * Coverage of a project's models by their assigned test cases, per feature
 * (06-ui.md: Coverage dashboard). Each model's coverage comes from
 * `GET /models/{id}/coverage`; there is no project-wide endpoint.
 */
@Component({
  selector: 'tm-coverage-dashboard',
  imports: [
    RouterLink, IxEmptyState, IxKpi, IxMessageBar, IxPill, IxProgressIndicator, IxSelect,
    IxSelectItem, IxSpinner,
  ],
  templateUrl: './coverage-dashboard.html',
  styleUrl: './coverage-dashboard.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CoverageDashboardComponent {
  private readonly explorer = inject(ExplorerStore);
  private readonly api = inject(OrgApi);

  readonly projects = this.explorer.projects;
  readonly projectId = signal<string | null>(null);
  readonly rows = signal<ModelRow[]>([]);
  readonly loading = signal(false);
  private requests = 0;

  readonly groups = computed(() => byFeature(this.rows()));
  readonly totals = computed(() => totals(this.rows()));
  readonly statePct = computed(() => percent(this.totals().states));
  readonly transitionPct = computed(() => percent(this.totals().transitions));

  readonly label = LEVEL_LABEL;
  readonly variant = LEVEL_VARIANT;

  constructor() {
    // Open on the first project, and move off one that was deleted.
    effect(() => {
      const projects = this.projects();
      const current = untracked(this.projectId);
      if (!projects.some(p => p.id === current)) this.projectId.set(projects[0]?.id ?? null);
    });
    // Reload when the project, or the models in its tree, change.
    effect(() => {
      const project = this.projects().find(p => p.id === this.projectId());
      const rows = modelRows(project);
      untracked(() => void this.load(rows));
    });
  }

  /** From `ix-select`'s `valueChange`, or a plain id. */
  setProject(value: Event | string): void {
    const raw = typeof value === 'string' ? value : (value as CustomEvent<string | string[]>).detail;
    const id = Array.isArray(raw) ? raw[0] : raw;
    if (id && this.projects().some(p => p.id === id)) this.projectId.set(id);
  }

  private async load(rows: ModelRow[]): Promise<void> {
    const request = ++this.requests;
    this.rows.set(rows);
    if (rows.length === 0) {
      this.loading.set(false);
      return;
    }
    this.loading.set(true);
    const loaded = await Promise.all(rows.map(async r => {
      try {
        return withCoverage(r, await this.api.modelCoverage(r.modelId));
      } catch (e) {
        return { ...r, error: e instanceof ApiError ? e.message : 'Coverage could not be loaded.' };
      }
    }));
    if (request !== this.requests) return;
    this.rows.set(loaded);
    this.loading.set(false);
  }

  pct(c: Count): number {
    return percent(c);
  }

  levelOf(r: ModelRow) {
    return r.states && r.transitions ? level(r.states, r.transitions) : null;
  }

  /** Progress status for a count: complete is success, anything else is partial. */
  status(c: Count): 'success' | 'warning' | 'base' {
    if (c.total === 0) return 'base';
    if (c.covered === c.total) return 'success';
    return c.covered === 0 ? 'base' : 'warning';
  }

  /** KPI state for a percentage, matching the status families above. */
  kpiState(p: number, total: number): 'success' | 'warning' | 'neutral' {
    if (total === 0 || p === 0) return 'neutral';
    return p === 100 ? 'success' : 'warning';
  }

  uncovered(r: ModelRow): string {
    if (!r.states || !r.transitions) return '';
    const s = r.states.total - r.states.covered;
    const t = r.transitions.total - r.transitions.covered;
    const parts = [];
    if (s) parts.push(`${s} ${s === 1 ? 'state' : 'states'}`);
    if (t) parts.push(`${t} ${t === 1 ? 'transition' : 'transitions'}`);
    return parts.length ? `${parts.join(', ')} uncovered` : 'Nothing uncovered';
  }
}
