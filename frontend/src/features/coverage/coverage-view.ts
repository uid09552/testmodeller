import { Coverage } from '../../core/api/org-api';
import { ExplorerProject } from '../explorer/state/explorer.store';

/** How far a model, feature or component is covered. Always shown as text too. */
export type CoverageLevel = 'covered' | 'partial' | 'none' | 'empty';

export interface Count { covered: number; total: number }

/** One model's coverage, or why it is missing. */
export interface ModelRow {
  modelId: string;
  model: string;
  featureId: string;
  feature: string;
  componentId: string;
  component: string;
  /** Absent until loaded, or when loading failed (`error`). */
  states?: Count;
  transitions?: Count;
  error?: string;
}

/** Totals over a set of rows; rows without coverage are left out. */
export interface Totals {
  states: Count;
  transitions: Count;
  models: number;
  covered: number;
  untested: number;
  failed: number;
}

/** Every model of a project, in tree order, with nothing loaded yet. */
export function modelRows(project: ExplorerProject | undefined): ModelRow[] {
  return (project?.components ?? []).flatMap(c =>
    c.features.flatMap(f =>
      f.models.map(m => ({
        modelId: m.id, model: m.name,
        featureId: f.id, feature: f.name,
        componentId: c.id, component: c.name,
      }))));
}

/** A row with its coverage filled in. */
export function withCoverage(row: ModelRow, coverage: Coverage): ModelRow {
  return { ...row, states: coverage.states, transitions: coverage.transitions, error: undefined };
}

/** Percentage of `c`, rounded; 0 when there is nothing to cover. */
export function percent(c: Count): number {
  return c.total ? Math.round((c.covered / c.total) * 100) : 0;
}

/** The level of a state and transition count together. */
export function level(states: Count, transitions: Count): CoverageLevel {
  const covered = states.covered + transitions.covered;
  const total = states.total + transitions.total;
  if (total === 0) return 'empty';
  if (covered === total) return 'covered';
  return covered === 0 ? 'none' : 'partial';
}

export const LEVEL_LABEL: Record<CoverageLevel, string> = {
  covered: 'Covered',
  partial: 'Partly covered',
  none: 'Not covered',
  empty: 'Empty model',
};

/** iX status family per level (DESIGN.md: covered = success, partial = warning, unknown = neutral). */
export const LEVEL_VARIANT: Record<CoverageLevel, 'success' | 'warning' | 'neutral'> = {
  covered: 'success',
  partial: 'warning',
  none: 'neutral',
  empty: 'neutral',
};

export function totals(rows: ModelRow[]): Totals {
  const sum = (pick: (r: ModelRow) => Count | undefined): Count =>
    rows.reduce((acc, r) => {
      const c = pick(r);
      return c ? { covered: acc.covered + c.covered, total: acc.total + c.total } : acc;
    }, { covered: 0, total: 0 });
  const loaded = rows.filter(r => r.states && r.transitions);
  const levels = loaded.map(r => level(r.states!, r.transitions!));
  return {
    states: sum(r => r.states),
    transitions: sum(r => r.transitions),
    models: rows.length,
    covered: levels.filter(l => l === 'covered').length,
    untested: levels.filter(l => l === 'none').length,
    failed: rows.filter(r => r.error).length,
  };
}

/** Feature subtotals, in row order, for the per-feature view the spec asks for. */
export interface FeatureGroup {
  featureId: string;
  feature: string;
  component: string;
  rows: ModelRow[];
  totals: Totals;
}

export function byFeature(rows: ModelRow[]): FeatureGroup[] {
  const groups = new Map<string, FeatureGroup>();
  for (const r of rows) {
    let g = groups.get(r.featureId);
    if (!g) {
      g = { featureId: r.featureId, feature: r.feature, component: r.component, rows: [], totals: totals([]) };
      groups.set(r.featureId, g);
    }
    g.rows.push(r);
  }
  for (const g of groups.values()) g.totals = totals(g.rows);
  return [...groups.values()];
}
