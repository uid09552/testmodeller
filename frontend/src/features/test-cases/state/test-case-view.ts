import { StateTest, TestCategory, TestPolarity } from '../../models/state/model-editor.store';

/**
 * Pure view logic for the cross-project Test Cases page: filtering, sorting
 * and grouping of table rows. Kept free of Angular so it is unit-testable.
 */

/** One row of the cross-project test case table. */
export interface TestRow {
  projectId: string;
  projectName: string;
  componentId: string;
  componentName: string;
  featureId: string;
  featureName: string;
  modelId: string;
  modelName: string;
  stateLabel: string;
  nodeId: string;
  ref: string;
  test: StateTest;
}

export type LinkFilter = 'all' | 'implemented' | 'unimplemented' | 'backlog' | 'none';

export interface TestCaseFilters {
  projectId: string;    // 'all' or an id
  componentId: string;  // 'all' or an id
  featureId: string;    // 'all' or an id
  modelId: string;      // 'all' or an id
  categories: TestCategory[];  // empty = every category
  polarity: 'all' | TestPolarity;
  links: LinkFilter;
  search: string;
}

export const EMPTY_FILTERS: TestCaseFilters = {
  projectId: 'all', componentId: 'all', featureId: 'all', modelId: 'all',
  categories: [], polarity: 'all', links: 'all', search: '',
};

export type SortKey = 'ref' | 'component' | 'feature' | 'model' | 'name' | 'category' | 'polarity';
export type SortDir = 'asc' | 'desc';
export interface SortState { key: SortKey; dir: SortDir }

export type GroupKey =
  'none' | 'project' | 'component' | 'feature' | 'model' | 'state' | 'category' | 'polarity';

export interface RowGroup {
  key: string;
  label: string;
  /** Hierarchy above the group, e.g. `Project › Component` for a feature group. */
  context: string;
  rows: TestRow[];
  positive: number;
  negative: number;
}

export const CATEGORY_ORDER: TestCategory[] = ['unit', 'integration', 'feature'];
const POLARITY_ORDER: TestPolarity[] = ['positive', 'negative'];

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

// ── Filtering ────────────────────────────────────────────────────────────────

function matchesLinks(t: StateTest, f: LinkFilter): boolean {
  const impl = !!t.implementationUrl?.trim();
  const backlog = !!t.backlogUrl?.trim();
  switch (f) {
    case 'all':           return true;
    case 'implemented':   return impl;
    case 'unimplemented': return !impl;
    case 'backlog':       return backlog;
    case 'none':          return !impl && !backlog;
  }
}

function matchesSearch(r: TestRow, q: string): boolean {
  if (!q) return true;
  const t = r.test;
  return [
    r.ref, t.name, r.modelName, r.stateLabel, r.featureName, r.componentName, r.projectName,
    t.given, t.when, t.then,
  ].some(s => s.toLowerCase().includes(q));
}

export function filterRows(rows: TestRow[], f: TestCaseFilters): TestRow[] {
  const q = f.search.toLowerCase().trim();
  return rows.filter(r =>
    (f.projectId   === 'all' || r.projectId   === f.projectId) &&
    (f.componentId === 'all' || r.componentId === f.componentId) &&
    (f.featureId   === 'all' || r.featureId   === f.featureId) &&
    (f.modelId     === 'all' || r.modelId     === f.modelId) &&
    (f.categories.length === 0 || f.categories.includes(r.test.category)) &&
    (f.polarity    === 'all' || r.test.polarity === f.polarity) &&
    matchesLinks(r.test, f.links) &&
    matchesSearch(r, q),
  );
}

/** Number of filters narrowing the result, for the "Clear filters" affordance. */
export function activeFilterCount(f: TestCaseFilters): number {
  return [
    f.projectId !== 'all', f.componentId !== 'all', f.featureId !== 'all', f.modelId !== 'all',
    f.categories.length > 0, f.polarity !== 'all', f.links !== 'all', f.search.trim() !== '',
  ].filter(Boolean).length;
}

export interface FacetOption { id: string; name: string; count: number }

/**
 * Options for the hierarchy dropdowns. Each level only offers values inside
 * the levels above it, so picking a project narrows the component list, etc.
 */
export function facetOptions(rows: TestRow[], f: TestCaseFilters) {
  const collect = (
    list: TestRow[], id: (r: TestRow) => string, name: (r: TestRow) => string,
  ): FacetOption[] => {
    const map = new Map<string, FacetOption>();
    for (const r of list) {
      const o = map.get(id(r)) ?? { id: id(r), name: name(r), count: 0 };
      o.count++;
      map.set(o.id, o);
    }
    return [...map.values()].sort((a, b) => collator.compare(a.name, b.name));
  };
  const inProject   = rows.filter(r => f.projectId === 'all' || r.projectId === f.projectId);
  const inComponent = inProject.filter(r => f.componentId === 'all' || r.componentId === f.componentId);
  const inFeature   = inComponent.filter(r => f.featureId === 'all' || r.featureId === f.featureId);
  return {
    components: collect(inProject,   r => r.componentId, r => r.componentName),
    features:   collect(inComponent, r => r.featureId,   r => r.featureName),
    models:     collect(inFeature,   r => r.modelId,     r => r.modelName),
  };
}

// ── Sorting ──────────────────────────────────────────────────────────────────

function compareBy(key: SortKey, a: TestRow, b: TestRow): number {
  switch (key) {
    case 'ref':
      return collator.compare(a.modelName, b.modelName) || a.test.seq - b.test.seq;
    case 'component': return collator.compare(a.componentName, b.componentName);
    case 'feature':   return collator.compare(a.featureName, b.featureName);
    case 'model':
      return collator.compare(a.modelName, b.modelName) || collator.compare(a.stateLabel, b.stateLabel);
    case 'name':      return collator.compare(a.test.name, b.test.name);
    case 'category':
      return CATEGORY_ORDER.indexOf(a.test.category) - CATEGORY_ORDER.indexOf(b.test.category);
    case 'polarity':
      return POLARITY_ORDER.indexOf(a.test.polarity) - POLARITY_ORDER.indexOf(b.test.polarity);
  }
}

/** Stable sort; ties fall back to the test id order (model, then sequence). */
export function sortRows(rows: TestRow[], s: SortState): TestRow[] {
  const sign = s.dir === 'asc' ? 1 : -1;
  return [...rows].sort((a, b) =>
    sign * compareBy(s.key, a, b) || compareBy('ref', a, b));
}

/** Clicking a header sorts by it ascending; clicking the active one flips direction. */
export function toggleSort(current: SortState, key: SortKey): SortState {
  return current.key === key
    ? { key, dir: current.dir === 'asc' ? 'desc' : 'asc' }
    : { key, dir: 'asc' };
}

// ── Grouping ─────────────────────────────────────────────────────────────────

function groupKeyOf(g: Exclude<GroupKey, 'none'>, r: TestRow): Omit<RowGroup, 'rows' | 'positive' | 'negative'> {
  switch (g) {
    case 'project':   return { key: r.projectId, label: r.projectName, context: '' };
    case 'component': return { key: r.componentId, label: r.componentName, context: r.projectName };
    case 'feature':
      return { key: r.featureId, label: r.featureName, context: `${r.projectName} › ${r.componentName}` };
    case 'model':
      return { key: r.modelId, label: r.modelName, context: `${r.componentName} › ${r.featureName}` };
    case 'state':
      return { key: `${r.modelId}/${r.nodeId}`, label: r.stateLabel, context: r.modelName };
    case 'category':  return { key: r.test.category, label: r.test.category, context: '' };
    case 'polarity':  return { key: r.test.polarity, label: r.test.polarity, context: '' };
  }
}

function compareGroups(g: GroupKey, a: RowGroup, b: RowGroup): number {
  if (g === 'category') {
    return CATEGORY_ORDER.indexOf(a.key as TestCategory) - CATEGORY_ORDER.indexOf(b.key as TestCategory);
  }
  if (g === 'polarity') {
    return POLARITY_ORDER.indexOf(a.key as TestPolarity) - POLARITY_ORDER.indexOf(b.key as TestPolarity);
  }
  return collator.compare(a.context, b.context) || collator.compare(a.label, b.label);
}

/**
 * Buckets already-sorted rows. Row order within a group is preserved, so the
 * column sort applies inside each group. `none` yields a single group.
 */
export function groupRows(rows: TestRow[], g: GroupKey): RowGroup[] {
  const groups = new Map<string, RowGroup>();
  for (const r of rows) {
    const head = g === 'none' ? { key: 'all', label: '', context: '' } : groupKeyOf(g, r);
    let group = groups.get(head.key);
    if (!group) {
      group = { ...head, rows: [], positive: 0, negative: 0 };
      groups.set(head.key, group);
    }
    group.rows.push(r);
    if (r.test.polarity === 'positive') group.positive++; else group.negative++;
  }
  return [...groups.values()].sort((a, b) => compareGroups(g, a, b));
}
