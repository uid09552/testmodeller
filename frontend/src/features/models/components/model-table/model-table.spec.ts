import { TestBed } from '@angular/core/testing';
import { ModelEditorStore } from '../../state/model-editor.store';
import { ModelTableComponent } from './model-table';

describe('ModelTableComponent', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<ModelTableComponent>>;
  let root: HTMLElement;
  let ids: { a: string; b: string; e: string };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ModelTableComponent], providers: [ModelEditorStore],
    }).compileComponents();
    store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    store.updateNode(a.id, { label: 'Start' });
    const b = store.addNode('regular', 300, 0);
    store.updateNode(b.id, { label: 'Home' });
    const e = store.addEdge(a.id, b.id);
    store.updateEdge(e.id, { label: 'login' });
    ids = { a: a.id, b: b.id, e: e.id };
    fixture = TestBed.createComponent(ModelTableComponent);
    fixture.detectChanges();
    root = fixture.nativeElement;
    document.body.appendChild(root);
  });

  afterEach(() => root.remove());

  const settle = async () => { await new Promise(r => setTimeout(r)); fixture.detectChanges(); };
  const cell = (grid: string, r: number, c: number) => root.querySelector<HTMLElement>(`#mt-${grid}-${r}-${c}`)!;
  const key = (target: Element, k: string, extra: KeyboardEventInit = {}) =>
    target.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, ...extra }));

  it('lists every state and transition with headers', () => {
    expect(root.querySelectorAll('[role="grid"]').length).toBe(2);
    expect(cell('states', 0, 0).textContent!.trim()).toBe('Start');
    expect(cell('states', 1, 1).textContent!.trim()).toBe('State');
    expect([...root.querySelectorAll('[role="grid"]')[1].querySelectorAll('th')].map(t => t.textContent))
      .toEqual(['From', 'Event', 'Guard', 'Action', 'Expected result', 'To', 'Issues']);
    expect(cell('transitions', 0, 0).textContent!.trim()).toBe('Start');
    expect(cell('transitions', 0, 5).textContent!.trim()).toBe('Home');
  });

  it('edits a guard with the keyboard alone, shares undo, and keeps focus on the cell', async () => {
    // Tab lands on the single tab stop; arrows move to the guard cell.
    const first = root.querySelector<HTMLElement>('[role="gridcell"][tabindex="0"]')!;
    first.focus();
    fixture.componentInstance.focusCell('transitions', 0, 0);
    await settle();
    key(cell('transitions', 0, 0), 'ArrowRight');
    await settle();
    key(document.activeElement!, 'ArrowRight');
    await settle();
    expect(document.activeElement).toBe(cell('transitions', 0, 2));
    expect(cell('transitions', 0, 2).getAttribute('tabindex')).toBe('0');

    key(cell('transitions', 0, 2), 'Enter');
    await settle();
    const input = root.querySelector<HTMLInputElement>('#mt-transitions-0-2-edit')!;
    expect(document.activeElement).toBe(input);
    input.value = 'x > 0';
    input.dispatchEvent(new Event('input'));
    key(input, 'Enter');
    await settle();

    expect(store.edgeById(ids.e)!.guard).toBe('x > 0');
    expect(document.activeElement).toBe(cell('transitions', 0, 2));
    key(document.body, 'z', { ctrlKey: true });
    expect(store.edgeById(ids.e)!.guard).toBeUndefined();
  });

  it('cancels an edit with Esc', async () => {
    fixture.componentInstance.startEdit('states', 0, 0);
    await settle();
    const input = root.querySelector<HTMLInputElement>('#mt-states-0-0-edit')!;
    input.value = 'Renamed';
    input.dispatchEvent(new Event('input'));
    key(input, 'Escape');
    await settle();
    expect(store.nodeById(ids.a)!.label).toBe('Start');
  });

  it('changes source and target from lists, and edits the expected result', async () => {
    const c = fixture.componentInstance;
    c.startEdit('transitions', 0, 5);
    await settle();
    const select = root.querySelector<HTMLSelectElement>('#mt-transitions-0-5-edit')!;
    select.value = ids.a;
    select.dispatchEvent(new Event('change'));
    key(select, 'Enter');
    expect(store.edgeById(ids.e)!.toId).toBe(ids.a);

    c.startEdit('transitions', 0, 4);
    c.draft('home is shown');
    c.finishEdit(true);
    expect(store.edgeById(ids.e)!.expected).toBe('home is shown');
  });

  it('adds a transition and a state, which appear in the model', async () => {
    const add = (text: string) =>
      [...root.querySelectorAll<HTMLButtonElement>('button')].find(b => b.textContent!.includes(text))!.click();
    add('Add transition');
    add('Add state');
    fixture.detectChanges();
    expect(store.edges().length).toBe(2);
    expect(store.nodes().length).toBe(3);
  });

  it('deletes a state on a second Delete, with its transitions, and shows issues per row', async () => {
    store.addNode('regular', 0, 300);   // unreachable: a validation issue
    fixture.detectChanges();
    expect(cell('states', 2, 0).closest('tr')!.querySelector('.mt__issue')!.textContent).toContain('unreachable');
    expect(cell('states', 2, 0).closest('tr')!.getAttribute('aria-describedby')).toContain('mt-issue-');

    key(cell('states', 1, 0), 'Delete');
    fixture.detectChanges();
    expect(store.nodes().length).toBe(3);
    expect(root.querySelector('[role="alert"]')!.textContent).toContain('Press Delete again');
    key(cell('states', 1, 0), 'Delete');
    fixture.detectChanges();
    expect(store.nodes().length).toBe(2);
    expect(store.edges().length).toBe(0);
  });
});
