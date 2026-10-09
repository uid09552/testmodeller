import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { Traceability, TraceabilityApi, TraceQuery } from '../../../core/api/traceability-api';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { TraceabilityPageComponent } from './traceability-page';

const TRACE: Traceability = {
  items: [
    {
      backlogUrl: 'https://jira.example/TM-1',
      testCases: [
        { id: 'a', name: 'Valid login', featureId: 'f1', componentId: 'c1',
          implementationUrl: 'https://git.example/login.spec.ts',
          elements: [{ modelId: 'm', stateId: 's' }] },
        { id: 'b', name: 'Wrong password', featureId: 'f1', componentId: 'c1', elements: [] },
      ],
    },
    {
      backlogUrl: 'javascript:alert(1)',
      testCases: [{ id: 'c', name: 'Odd', featureId: 'f1', componentId: 'c1', elements: [] }],
    },
  ],
  untraced: [{ id: 'd', name: 'Orphan', featureId: 'f1', componentId: 'c1', elements: [] }],
  summary: { backlogItems: 2, testCases: 4, untraced: 1, unimplemented: 2, noElements: 1 },
};

describe('TraceabilityPageComponent', () => {
  let queries: { projectId: string; query: TraceQuery }[];
  let root: HTMLElement;
  let fixture: ReturnType<typeof TestBed.createComponent<TraceabilityPageComponent>>;

  async function settle(): Promise<void> {
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(async () => {
    queries = [];
    const projects = signal([{
      id: 'p1', name: 'Shop', expanded: true,
      components: [{
        id: 'c1', name: 'Auth', expanded: true,
        features: [{ id: 'f1', name: 'Login', description: '', models: [], expanded: true }],
      }],
    }]);
    await TestBed.configureTestingModule({
      imports: [TraceabilityPageComponent],
      providers: [
        provideRouter([]),
        { provide: ExplorerStore, useValue: { projects } },
        {
          provide: TraceabilityApi,
          useValue: {
            get: async (projectId: string, query: TraceQuery) => {
              queries.push({ projectId, query });
              return TRACE;
            },
          },
        },
      ],
    }).compileComponents();
    fixture = TestBed.createComponent(TraceabilityPageComponent);
    fixture.detectChanges();
    await settle();
    root = fixture.nativeElement;
  });

  it('loads the first project and groups test cases under their backlog item', () => {
    expect(queries[0]).toEqual({ projectId: 'p1', query: expect.objectContaining({}) });
    expect(root.textContent).toContain('Valid login');
    expect(root.textContent).toContain('Wrong password');
    const cell = root.querySelector<HTMLTableCellElement>('td.item')!;
    expect(cell.rowSpan).toBe(2);
  });

  it('names component and scenario, and flags a missing implementation', () => {
    expect(root.textContent).toContain('Auth › Login');
    expect(root.textContent).toContain('Not implemented');
  });

  it('opens links in a new tab without handing over the opener', () => {
    const link = root.querySelector<HTMLAnchorElement>('a[href="https://git.example/login.spec.ts"]')!;
    expect(link.target).toBe('_blank');
    expect(link.rel).toContain('noopener');
    expect(link.rel).toContain('noreferrer');
  });

  it('does not make a link of a URL that is not http(s)', () => {
    expect(root.querySelector('a[href^="javascript:"]')).toBeNull();
    expect(root.textContent).toContain('javascript:alert(1)');
  });

  it('opens a test case in the editor through the test cases page', () => {
    const link = root.querySelector<HTMLAnchorElement>('a[href*="/test-cases"]')!;
    expect(link.getAttribute('href')).toBe('/test-cases?open=a');
  });

  it('flags a backlog item that covers no model element, and untraced test cases', () => {
    expect(root.textContent).toContain('No model element');
    expect(root.textContent).toContain('No backlog item');
    expect(root.textContent).toContain('Orphan');
  });

  it('narrows by search text', async () => {
    const input = root.querySelector<HTMLInputElement>('input[aria-label="Search the trace"]')!;
    input.value = 'wrong';
    input.dispatchEvent(new Event('input'));
    await settle();
    expect(root.textContent).toContain('Wrong password');
    expect(root.textContent).not.toContain('Valid login');
    expect(root.textContent).not.toContain('Orphan');
  });

  it('asks the server for the chosen gap', async () => {
    const select = root.querySelector<HTMLSelectElement>('select[aria-label="Gap"]')!;
    select.value = 'unimplemented';
    select.dispatchEvent(new Event('change'));
    await settle();
    expect(queries.at(-1)?.query.gap).toBe('unimplemented');
  });

  it('shows the gap counts on the gaps tab and selects a gap', async () => {
    const tab = [...root.querySelectorAll<HTMLButtonElement>('.tabs .chip')]
      .find(b => b.textContent?.includes('Gaps'))!;
    expect(tab.textContent).toContain('4');
    tab.click();
    await settle();
    expect(queries.at(-1)?.query.gap).toBe('untraced');
    const cards = [...root.querySelectorAll<HTMLButtonElement>('.stat--button')];
    expect(cards.map(c => [
      c.querySelector('.stat__label')?.textContent, c.querySelector('.stat__value')?.textContent,
    ])).toEqual([
      ['No backlog item', '1'], ['No implementation', '2'], ['No model element', '1'],
    ]);
    cards[2].click();
    await settle();
    expect(queries.at(-1)?.query.gap).toBe('no-elements');
  });

  it('narrows to a component and a scenario', async () => {
    const component = root.querySelector<HTMLSelectElement>('select[aria-label="Component"]')!;
    component.value = 'c1';
    component.dispatchEvent(new Event('change'));
    await settle();
    expect(queries.at(-1)?.query.componentId).toBe('c1');
    const feature = root.querySelector<HTMLSelectElement>('select[aria-label="Scenario"]')!;
    feature.value = 'f1';
    feature.dispatchEvent(new Event('change'));
    await settle();
    expect(queries.at(-1)?.query.featureId).toBe('f1');
  });
});
