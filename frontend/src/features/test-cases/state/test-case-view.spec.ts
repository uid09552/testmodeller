import { StateTest } from '../../models/state/model-editor.store';
import {
  EMPTY_FILTERS, TestRow, activeFilterCount, facetOptions, filterRows, groupRows, sortRows,
  toggleSort,
} from './test-case-view';

function row(over: Partial<Omit<TestRow, 'test'>> & { test?: Partial<StateTest> } = {}): TestRow {
  const { test, ...rest } = over;
  const t: StateTest = {
    id: 't1', seq: 1, name: 'Login works', category: 'unit', polarity: 'positive',
    given: 'a user', when: 'they log in', then: 'they see the dashboard', ...test,
  };
  return {
    projectId: 'p1', projectName: 'Shop', componentId: 'c1', componentName: 'Auth',
    featureId: 'f1', featureName: 'Login', modelId: 'm1', modelName: 'LoginFlow',
    stateLabel: 'Start', nodeId: 'n1', ref: `LoginFlow_${t.seq}`, ...rest, test: t,
  };
}

describe('test case view (FR-004)', () => {
  const rows = [
    row({ test: { id: 'a', seq: 10, name: 'Zeta', category: 'feature', polarity: 'negative' } }),
    row({ test: { id: 'b', seq: 2, name: 'alpha', implementationUrl: 'https://x/y' } }),
    row({
      componentId: 'c2', componentName: 'Cart', featureId: 'f2', featureName: 'Checkout',
      modelId: 'm2', modelName: 'CheckoutFlow', stateLabel: 'Paid',
      test: { id: 'c', seq: 1, name: 'Pay', category: 'integration', backlogUrl: 'https://jira/1' },
    }),
  ];
  const ids = (rs: TestRow[]) => rs.map(r => r.test.id);

  it('filters by hierarchy, categories, polarity and links', () => {
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, componentId: 'c2' }))).toEqual(['c']);
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, categories: ['unit', 'feature'] }))).toEqual(['a', 'b']);
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, polarity: 'negative' }))).toEqual(['a']);
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, links: 'implemented' }))).toEqual(['b']);
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, links: 'backlog' }))).toEqual(['c']);
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, links: 'none' }))).toEqual(['a']);
  });

  it('searches state names and Gherkin steps', () => {
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, search: 'paid' }))).toEqual(['c']);
    expect(ids(filterRows(rows, { ...EMPTY_FILTERS, search: 'DASHBOARD' }))).toEqual(['a', 'b', 'c']);
  });

  it('counts active filters', () => {
    expect(activeFilterCount(EMPTY_FILTERS)).toBe(0);
    expect(activeFilterCount({ ...EMPTY_FILTERS, categories: ['unit'], search: ' x ' })).toBe(2);
  });

  it('narrows facet options by the levels above', () => {
    const f = facetOptions(rows, { ...EMPTY_FILTERS, componentId: 'c1' });
    expect(f.components.map(o => o.name)).toEqual(['Auth', 'Cart']);
    expect(f.features.map(o => o.name)).toEqual(['Login']);
    expect(f.models).toEqual([{ id: 'm1', name: 'LoginFlow', count: 2 }]);
  });

  it('sorts ids numerically within a model and flips direction', () => {
    const asc = { key: 'ref' as const, dir: 'asc' as const };
    expect(ids(sortRows(rows, asc))).toEqual(['c', 'b', 'a']);
    expect(ids(sortRows(rows, toggleSort(asc, 'ref')))).toEqual(['a', 'b', 'c']);
    expect(toggleSort(asc, 'name')).toEqual({ key: 'name', dir: 'asc' });
    expect(ids(sortRows(rows, { key: 'name', dir: 'asc' }))).toEqual(['b', 'c', 'a']);
    expect(ids(sortRows(rows, { key: 'category', dir: 'asc' }))).toEqual(['b', 'c', 'a']);
  });

  it('groups rows keeping their order and counting polarity', () => {
    const groups = groupRows(sortRows(rows, { key: 'ref', dir: 'asc' }), 'component');
    expect(groups.map(g => g.label)).toEqual(['Auth', 'Cart']);
    expect(ids(groups[0].rows)).toEqual(['b', 'a']);
    expect([groups[0].positive, groups[0].negative]).toEqual([1, 1]);

    expect(groupRows(rows, 'category').map(g => g.key)).toEqual(['unit', 'integration', 'feature']);
    expect(groupRows(rows, 'none')).toHaveLength(1);
  });
});
