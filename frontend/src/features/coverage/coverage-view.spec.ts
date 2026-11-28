import { ExplorerProject } from '../explorer/state/explorer.store';
import { byFeature, level, ModelRow, modelRows, percent, totals, withCoverage } from './coverage-view';

const PROJECT: ExplorerProject = {
  id: 'p1', name: 'Webshop', expanded: true,
  components: [{
    id: 'c1', name: 'Checkout', expanded: true,
    features: [
      { id: 'f1', name: 'Cart', description: '', expanded: true, models: [
        { id: 'm1', name: 'Add item', status: 'draft', states: 3, transitions: 2 },
        { id: 'm2', name: 'Remove item', status: 'draft', states: 2, transitions: 1 },
      ] },
      { id: 'f2', name: 'Pay', description: '', expanded: true, models: [
        { id: 'm3', name: 'Card', status: 'draft', states: 4, transitions: 4 },
      ] },
    ],
  }],
};

const cov = (s: [number, number], t: [number, number]) => ({
  states: { covered: s[0], total: s[1] }, transitions: { covered: t[0], total: t[1] },
});

describe('coverage view', () => {
  it('lists every model of the project in tree order', () => {
    const rows = modelRows(PROJECT);
    expect(rows.map(r => `${r.component}/${r.feature}/${r.model}`))
      .toEqual(['Checkout/Cart/Add item', 'Checkout/Cart/Remove item', 'Checkout/Pay/Card']);
    expect(modelRows(undefined)).toEqual([]);
  });

  it('names the level of coverage', () => {
    expect(level({ covered: 3, total: 3 }, { covered: 2, total: 2 })).toBe('covered');
    expect(level({ covered: 1, total: 3 }, { covered: 0, total: 2 })).toBe('partial');
    expect(level({ covered: 0, total: 3 }, { covered: 0, total: 2 })).toBe('none');
    expect(level({ covered: 0, total: 0 }, { covered: 0, total: 0 })).toBe('empty');
    expect(percent({ covered: 1, total: 3 })).toBe(33);
    expect(percent({ covered: 0, total: 0 })).toBe(0);
  });

  it('totals loaded rows and counts the failed ones', () => {
    const [a, b, c] = modelRows(PROJECT);
    const rows: ModelRow[] = [
      withCoverage(a, cov([3, 3], [2, 2])),
      withCoverage(b, cov([0, 2], [0, 1])),
      { ...c, error: 'boom' },
    ];
    expect(totals(rows)).toEqual({
      states: { covered: 3, total: 5 }, transitions: { covered: 2, total: 3 },
      models: 3, covered: 1, untested: 1, failed: 1,
    });
  });

  it('groups by feature with subtotals', () => {
    const rows = modelRows(PROJECT).map(r => withCoverage(r, cov([1, 2], [1, 2])));
    const groups = byFeature(rows);
    expect(groups.map(g => [g.feature, g.rows.length, g.totals.states.total]))
      .toEqual([['Cart', 2, 4], ['Pay', 1, 2]]);
  });
});
