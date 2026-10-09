import { TraceItem, TraceTestCase } from '../../../core/api/traceability-api';
import { coversNoElement, searchTrace, toCsv, toRows, TraceNames } from './traceability-view';

function tc(partial: Partial<TraceTestCase> & { id: string }): TraceTestCase {
  return {
    name: 'Valid login', featureId: 'f1', componentId: 'c1', elements: [], ...partial,
  };
}

const ITEMS: TraceItem[] = [
  {
    backlogUrl: 'https://jira.example/TM-1',
    testCases: [
      tc({ id: 'a', name: 'Valid login', implementationUrl: 'https://git/login.spec.ts',
           elements: [{ modelId: 'm', stateId: 's' }] }),
      tc({ id: 'b', name: 'Wrong password' }),
    ],
  },
  { backlogUrl: 'https://jira.example/TM-2', testCases: [tc({ id: 'c', name: 'Reset' })] },
];
const UNTRACED = [tc({ id: 'd', name: 'Orphan' })];

const NAMES: TraceNames = {
  component: id => ({ c1: 'Auth' } as Record<string, string>)[id] ?? id,
  feature: id => ({ f1: 'Login' } as Record<string, string>)[id] ?? id,
};

describe('toRows', () => {
  it('lists each item and test case pair, then the untraced test cases', () => {
    const rows = toRows(ITEMS, UNTRACED);
    expect(rows.map(r => [r.backlogUrl, r.testCase.id])).toEqual([
      ['https://jira.example/TM-1', 'a'],
      ['https://jira.example/TM-1', 'b'],
      ['https://jira.example/TM-2', 'c'],
      ['', 'd'],
    ]);
  });
});

describe('searchTrace', () => {
  it('returns everything for a blank search', () => {
    expect(searchTrace(ITEMS, UNTRACED, '  ')).toEqual({ items: ITEMS, untraced: UNTRACED });
  });

  it('finds test cases by name, ignoring case, and drops items left empty', () => {
    const r = searchTrace(ITEMS, UNTRACED, 'WRONG');
    expect(r.items.map(i => i.backlogUrl)).toEqual(['https://jira.example/TM-1']);
    expect(r.items[0].testCases.map(t => t.id)).toEqual(['b']);
    expect(r.untraced).toEqual([]);
  });

  it('finds by backlog URL and by implementation URL', () => {
    expect(searchTrace(ITEMS, [], 'tm-2').items[0].testCases.map(t => t.id)).toEqual(['c']);
    expect(searchTrace(ITEMS, [], 'login.spec').items[0].testCases.map(t => t.id)).toEqual(['a']);
  });
});

describe('coversNoElement', () => {
  it('is true only when no test case of the item is assigned', () => {
    expect(coversNoElement(ITEMS[0])).toBe(false);
    expect(coversNoElement(ITEMS[1])).toBe(true);
  });
});

describe('toCsv (FR-028)', () => {
  it('has a header and one line per backlog item and test case pair', () => {
    const lines = toCsv(toRows(ITEMS, []), NAMES).trimEnd().split('\r\n');
    expect(lines).toHaveLength(1 + 3);
    expect(lines[0]).toBe(
      'Backlog item,Test case,Test case id,Component,Feature,Implementation,Model elements');
    expect(lines[1]).toBe(
      'https://jira.example/TM-1,Valid login,a,Auth,Login,https://git/login.spec.ts,1');
    expect(lines[2]).toBe('https://jira.example/TM-1,Wrong password,b,Auth,Login,,0');
    expect(lines[3]).toBe('https://jira.example/TM-2,Reset,c,Auth,Login,,0');
  });

  it('leaves the backlog cell empty for an untraced test case', () => {
    const lines = toCsv(toRows([], UNTRACED), NAMES).trimEnd().split('\r\n');
    expect(lines[1]).toBe(',Orphan,d,Auth,Login,,0');
  });

  it('quotes commas, quotes and line breaks', () => {
    const rows = toRows([], [tc({ id: 'x', name: 'Say "hi", then\nleave' })]);
    expect(toCsv(rows, NAMES)).toContain('"Say ""hi"", then\nleave"');
  });

  it('defuses cells a spreadsheet would run as a formula', () => {
    const rows = toRows([], [tc({ id: 'x', name: '=HYPERLINK("http://evil")' })]);
    const line = toCsv(rows, NAMES).split('\r\n')[1];
    expect(line.startsWith(`,"'=HYPERLINK(""http://evil"")"`)).toBe(true);
  });
});
