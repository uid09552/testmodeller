import { TraceItem, TraceTestCase } from '../../../core/api/traceability-api';

/** Names for the ids a trace carries, from the explorer tree. */
export interface TraceNames {
  component(id: string): string;
  feature(id: string): string;
}

/** One line of the matrix and of its CSV export: a test case under a backlog item, if any. */
export interface TraceRow {
  backlogUrl: string;
  testCase: TraceTestCase;
}

/** The rows of the items, then of the test cases without a backlog item. */
export function toRows(items: TraceItem[], untraced: TraceTestCase[]): TraceRow[] {
  return [
    ...items.flatMap(i => i.testCases.map(testCase => ({ backlogUrl: i.backlogUrl, testCase }))),
    ...untraced.map(testCase => ({ backlogUrl: '', testCase })),
  ];
}

/** Keeps the test cases whose name, or whose backlog or implementation URL, contains `text`. */
export function searchTrace(
  items: TraceItem[], untraced: TraceTestCase[], text: string,
): { items: TraceItem[]; untraced: TraceTestCase[] } {
  const q = text.trim().toLowerCase();
  if (!q) return { items, untraced };
  const hit = (tc: TraceTestCase, backlogUrl: string) =>
    [tc.name, tc.implementationUrl ?? '', backlogUrl].some(s => s.toLowerCase().includes(q));
  return {
    items: items
      .map(i => ({ ...i, testCases: i.testCases.filter(tc => hit(tc, i.backlogUrl)) }))
      .filter(i => i.testCases.length > 0),
    untraced: untraced.filter(tc => hit(tc, '')),
  };
}

/** True when no test case of the item is assigned to a state or transition. */
export function coversNoElement(item: TraceItem): boolean {
  return item.testCases.every(tc => tc.elements.length === 0);
}

const CSV_HEADER = [
  'Backlog item', 'Test case', 'Test case id', 'Component', 'Feature',
  'Implementation', 'Model elements', 'Latest result',
];

/** A CSV cell. Quoted when needed; a leading `= + - @` is defused so a spreadsheet will not run it. */
function cell(value: string): string {
  const safe = /^[=+\-@\t\r]/.test(value) ? `'${value}` : value;
  return /[",\r\n]/.test(safe) ? `"${safe.replace(/"/g, '""')}"` : safe;
}

/** CSV with one line per backlog item and test case pair (FR-028). */
export function toCsv(rows: TraceRow[], names: TraceNames): string {
  const lines = rows.map(({ backlogUrl, testCase: tc }) => [
    backlogUrl, tc.name, tc.id, names.component(tc.componentId), names.feature(tc.featureId),
    tc.implementationUrl ?? '', String(tc.elements.length), tc.lastResult?.status ?? '',
  ].map(cell).join(','));
  return [CSV_HEADER.map(cell).join(','), ...lines].join('\r\n') + '\r\n';
}
