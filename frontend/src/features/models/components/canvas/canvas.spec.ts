import { TestBed } from '@angular/core/testing';
import { CanvasComponent } from './canvas';
import { anchorPoint, ModelEditorStore } from '../../state/model-editor.store';

/** A left mouse-down at a screen position. */
function leftDown(x: number, y: number): MouseEvent {
  return new MouseEvent('mousedown', { button: 0, clientX: x, clientY: y, bubbles: true });
}

/** Presses a key as the browser would, on the element that has focus. */
function press(key: string, target: EventTarget = document.body): void {
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
}

describe('CanvasComponent keyboard', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    fixture.detectChanges();
  });

  it('deletes the selected state on Delete', () => {
    const node = store.addNode('regular', 0, 0);
    store.select(node.id, 'node');

    press('Delete');

    expect(store.nodes().length).toBe(0);
  });

  it('deletes a marquee selection of several states on Delete', () => {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 200, 0);
    store.selectNodes([a.id, b.id]);

    press('Delete');

    expect(store.nodes().length).toBe(0);
  });

  it('deletes the selected transition on Delete', () => {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 200, 0);
    const edge = store.addEdge(a.id, b.id);
    store.select(edge.id, 'edge');

    press('Delete');

    expect(store.edges().length).toBe(0);
    expect(store.nodes().length).toBe(2);
  });

  it('deletes with Backspace too', () => {
    const node = store.addNode('regular', 0, 0);
    store.select(node.id, 'node');

    press('Backspace');

    expect(store.nodes().length).toBe(0);
  });

  it('leaves the model alone while a text field has focus', () => {
    const node = store.addNode('regular', 0, 0);
    store.select(node.id, 'node');
    const input = document.createElement('input');
    document.body.appendChild(input);

    press('Delete', input);

    expect(store.nodes().length).toBe(1);
    input.remove();
  });

  it('leaves the model alone while a state label is being edited', () => {
    const node = store.addNode('regular', 0, 0);
    store.select(node.id, 'node');
    fixture.componentInstance.startEdit(node.id, node.label);

    press('Delete');

    expect(store.nodes().length).toBe(1);
  });
});

describe('CanvasComponent tools', () => {
  let store: ModelEditorStore;
  let canvas: CanvasComponent;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    canvas = fixture.componentInstance;
    store = TestBed.inject(ModelEditorStore);
    fixture.detectChanges();
  });

  it('starts on the select tool', () => {
    expect(canvas.tool()).toBe('select');
  });

  it('switches tool with V and H, as other canvas tools do', () => {
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'h', bubbles: true }));
    expect(canvas.tool()).toBe('pan');
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'v', bubbles: true }));
    expect(canvas.tool()).toBe('select');
  });

  it('does not steal V or H from a text field', () => {
    const input = document.createElement('input');
    document.body.appendChild(input);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'h', bubbles: true }));
    expect(canvas.tool()).toBe('select');
    input.remove();
  });

  it('moves a state with the select tool', () => {
    const node = store.addNode('regular', 100, 100);

    canvas.onNodeMouseDown(leftDown(10, 10), node);

    // A drag is in progress on the state, so the view has not moved.
    expect(canvas.panX()).toBe(40);
    expect(canvas.panY()).toBe(40);
  });

  it('pans instead of moving a state with the hand tool', () => {
    const node = store.addNode('regular', 100, 100);
    canvas.setTool('pan');

    canvas.onNodeMouseDown(leftDown(10, 10), node);
    document.dispatchEvent(
      new MouseEvent('mousemove', { clientX: 60, clientY: 90, bubbles: true }),
    );

    expect(canvas.panX()).toBe(90);   // 40 + (60 - 10)
    expect(canvas.panY()).toBe(120);  // 40 + (90 - 10)
    expect(store.nodes()[0].x).toBe(100);
    expect(store.nodes()[0].y).toBe(100);
  });

  it('shows the hand cursor while the pan tool is active', () => {
    expect(canvas.canvasCursor()).toBeNull();
    canvas.setTool('pan');
    expect(canvas.canvasCursor()).toBe('grab');
  });

  it('abandons a half-drawn transition when the tool changes', () => {
    const a = store.addNode('regular', 0, 0);
    canvas.onConnectorMouseDown(leftDown(0, 0), a, 'right');
    expect(canvas.drawing()).not.toBeNull();

    canvas.setTool('pan');

    expect(canvas.drawing()).toBeNull();
  });
});

describe('CanvasComponent state sizing', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    fixture.detectChanges();
  });

  it('grows a state around its centre when its name gets long', () => {
    const node = store.addNode('regular', 100, 100);
    const cx = node.x + node.w / 2;
    const cy = node.y + node.h / 2;
    store.updateNode(node.id, { label: 'User submits the registration form with valid credentials' });
    const grown = store.nodeById(node.id)!;
    expect(grown.h).toBeGreaterThan(node.h);
    expect(grown.x + grown.w / 2).toBeCloseTo(cx);
    expect(grown.y + grown.h / 2).toBeCloseTo(cy);
  });

  it('shrinks back to the default size, never below it', () => {
    const node = store.addNode('regular', 0, 0);
    store.updateNode(node.id, { label: 'User submits the registration form with valid credentials' });
    store.updateNode(node.id, { label: 'Ok' });
    const back = store.nodeById(node.id)!;
    expect([back.w, back.h]).toEqual([node.w, node.h]);
  });

  it('renders a long name as several lines and keeps transitions on the new outline', () => {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 400, 0);
    store.updateNode(a.id, { label: 'User submits the registration form with valid credentials' });
    fixture.detectChanges();
    const tspans = (fixture.nativeElement as HTMLElement).querySelectorAll('.node-group .node-label tspan');
    expect(tspans.length).toBeGreaterThan(2);
    const grown = store.nodeById(a.id)!;
    expect(anchorPoint(grown, 'right').x).toBe(grown.x + grown.w);
    expect(b.id).toBeTruthy();
  });
});

describe('CanvasComponent font loading', () => {
  it('re-fits every state once web fonts are ready', async () => {
    let ready!: () => void;
    Object.defineProperty(document, 'fonts', {
      configurable: true,
      value: { ready: new Promise<void>(r => { ready = r; }) },
    });
    try {
      await TestBed.configureTestingModule({
        imports: [CanvasComponent],
        providers: [ModelEditorStore],
      }).compileComponents();
      const store = TestBed.inject(ModelEditorStore);
      const refit = vi.spyOn(store, 'refitNodes');
      TestBed.createComponent(CanvasComponent).detectChanges();
      expect(refit).not.toHaveBeenCalled();
      ready();
      await Promise.resolve();
      await Promise.resolve();
      expect(refit).toHaveBeenCalledTimes(1);
    } finally {
      delete (document as { fonts?: unknown }).fonts;
    }
  });
});

describe('CanvasComponent validation navigation', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let root: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    fixture.detectChanges();
    root = fixture.nativeElement;
  });

  const badge = () => root.querySelector<HTMLElement>('.badge--status')!;

  it('opens the Validation tab, un-collapsing the panel, when the error badge is clicked', () => {
    store.bottomCollapsed.set(true);
    fixture.detectChanges();
    expect(badge().textContent).toContain('1 error');
    badge().click();
    expect(store.bottomTab()).toBe('validation');
    expect(store.bottomCollapsed()).toBe(false);
  });

  it('opens it from the warning badge too', () => {
    store.addNode('initial', 0, 0);
    fixture.detectChanges();
    expect(badge().textContent).toContain('warning');
    badge().click();
    expect(store.bottomTab()).toBe('validation');
  });

  it('shows a valid model as a non-interactive indicator', () => {
    const a = store.addNode('initial', 0, 0);
    const b = store.addNode('final', 300, 0);
    store.addEdge(a.id, b.id);
    fixture.detectChanges();
    expect(badge().tagName).not.toBe('BUTTON');
    badge().click();
    expect(store.bottomTab()).toBe('scenario');
  });

  it('selects a state for a state issue', () => {
    const a = store.addNode('initial', 0, 0);
    const issue = store.issues().find(i => i.elementId === a.id)!;
    store.revealIssue(issue);
    expect(store.selection()).toEqual([{ id: a.id, type: 'node' }]);
  });

  it('selects a transition, not a state, for a transition issue', () => {
    const a = store.addNode('initial', 0, 0);
    const b = store.addNode('final', 300, 0);
    const edge = store.addEdge(a.id, b.id)!;
    store.revealIssue({ code: 'X', severity: 'error', message: 'x', elementId: edge.id });
    expect(store.selection()).toEqual([{ id: edge.id, type: 'edge' }]);
  });

  it('ignores an issue with no element', () => {
    store.revealIssue({ code: 'X', severity: 'error', message: 'x' });
    expect(store.selection()).toEqual([]);
    expect(store.revealRequest()).toBeNull();
  });

  it('pans an off-screen element into view and leaves a visible one alone', () => {
    const near = store.addNode('regular', 100, 100);
    const far = store.addNode('regular', 5000, 5000);
    const c = fixture.componentInstance;
    vi.spyOn(c['svgEl']().nativeElement, 'getBoundingClientRect')
      .mockReturnValue({ width: 800, height: 600 } as DOMRect);
    c.panX.set(0); c.panY.set(0);
    c.revealElement(near.id);
    expect([c.panX(), c.panY()]).toEqual([0, 0]);
    c.revealElement(far.id);
    expect(far.x + far.w / 2 + c.panX()).toBeCloseTo(400, 0);
  });
});

describe('CanvasComponent multi-line state names', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let c: CanvasComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    c = fixture.componentInstance;
    fixture.detectChanges();
  });

  const key = (k: string, shiftKey = false) =>
    new KeyboardEvent('keydown', { key: k, shiftKey, cancelable: true });

  it('Shift+Enter leaves the event alone so the textarea inserts a break', () => {
    const node = store.addNode('regular', 0, 0);
    c.startEdit(node.id, node.label);
    const e = key('Enter', true);
    c.onEditKeydown(e);
    expect(e.defaultPrevented).toBe(false);
    expect(c.editNodeId()).toBe(node.id);
  });

  it('Enter commits the name with its line breaks and drops trailing blank lines', () => {
    const node = store.addNode('regular', 0, 0);
    c.startEdit(node.id, node.label);
    c.onEditInput('Login\nForm\n\n');
    c.onEditKeydown(key('Enter'));
    expect(store.nodeById(node.id)!.label).toBe('Login\nForm');
    expect(c.editNodeId()).toBeNull();
  });

  it('Escape restores the original name', () => {
    const node = store.addNode('regular', 0, 0);
    c.startEdit(node.id, 'State');
    c.onEditInput('Something else\nentirely');
    c.onEditKeydown(key('Escape'));
    expect(store.nodeById(node.id)!.label).toBe('State');
    expect(c.editNodeId()).toBeNull();
  });

  it('resizes the state live as breaks are added and removed', () => {
    const node = store.addNode('regular', 0, 0);
    c.startEdit(node.id, node.label);
    c.onEditInput('A\nB\nC\nD');
    const grown = store.nodeById(node.id)!;
    expect(grown.h).toBeGreaterThan(node.h);
    expect(c.editBox(grown).h).toBeLessThanOrEqual(grown.h);
    c.onEditInput('A');
    expect(store.nodeById(node.id)!.h).toBe(node.h);
  });

  it('draws each line of the name as its own line', () => {
    const node = store.addNode('regular', 0, 0);
    store.updateNode(node.id, { label: 'One\nTwo\nThree' });
    fixture.detectChanges();
    const lines = Array.from((fixture.nativeElement as HTMLElement).querySelectorAll('.node-label tspan'))
      .map(t => t.textContent!.trim());
    expect(lines).toEqual(['One', 'Two', 'Three']);
  });
});

describe('multi-line names outside the canvas', () => {
  it('shows them on one line in validation messages and test labels', async () => {
    await TestBed.configureTestingModule({ providers: [ModelEditorStore] }).compileComponents();
    const store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    store.updateNode(a.id, { label: 'Login\nform' });
    expect(store.issues().some(i => i.message.includes('"Login form"'))).toBe(true);
    expect(store.issues().every(i => !i.message.includes('\n'))).toBe(true);
  });
});

describe('CanvasComponent coverage overlay', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let root: HTMLElement;

  const toolbarButton = () => [...root.querySelectorAll<HTMLButtonElement>('.tb-coverage .tb-btn')][0];
  const counts = () => root.querySelector('.tb-coverage__counts')?.textContent?.replace(/\s+/g, ' ').trim();
  const nodeGroup = (id: string) =>
    [...root.querySelectorAll<SVGGElement>('g.node-group')]
      .find(g => g.getAttribute('aria-label')?.includes(store.nodeById(id)!.label))!;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    root = fixture.nativeElement;
  });

  function model() {
    const a = store.addNode('initial', 0, 0);
    const b = store.addNode('regular', 300, 0);
    store.updateNode(a.id, { label: 'Start' });
    store.updateNode(b.id, { label: 'Home' });
    store.addTest(a.id);
    const e1 = store.addEdge(a.id, b.id);
    const e2 = store.addEdge(b.id, a.id);
    store.setTransitionCoverage({ covered: 1, total: 2, uncoveredTransitionIds: [e2.id] });
    store.clearDirty();
    fixture.detectChanges();
    return { a, b, e1, e2 };
  }

  it('is off by default and leaves the canvas as it is', () => {
    model();
    expect(root.querySelector('.node-group--uncovered')).toBeNull();
    expect(root.querySelector('.edge-group--uncovered')).toBeNull();
    expect(counts()).toBeUndefined();
    expect(root.querySelector('.node-gap-outline')).toBeNull();
  });

  it('shows both dimensions with counts when switched on', () => {
    const { b, e2 } = model();
    toolbarButton().click();
    fixture.detectChanges();

    expect(counts()).toBe('1/2 states, 1/2 transitions');
    expect(root.querySelectorAll('.node-group--uncovered')).toHaveLength(1);
    expect(nodeGroup(b.id).classList).toContain('node-group--uncovered');
    expect(root.querySelectorAll('.edge-group--uncovered')).toHaveLength(1);
    const hit = [...root.querySelectorAll('path.edge-hit')]
      .find(p => p.getAttribute('aria-label')?.includes('not covered'));
    expect(hit).toBeTruthy();
    expect(store.isUncoveredTransition(e2.id)).toBe(true);
  });

  it('marks an uncovered state with a non-colour cue and an accessible label', () => {
    const { b } = model();
    store.coverageView.set('states');
    fixture.detectChanges();
    const g = nodeGroup(b.id);
    expect(g.querySelector('.node-gap-outline')).not.toBeNull();
    expect(g.querySelector('.node-gap-marker text')?.textContent).toBe('0');
    expect(g.getAttribute('aria-label')).toBe('regular state: Home, not covered by any test case');
  });

  it('shows states only', () => {
    model();
    store.coverageView.set('states');
    fixture.detectChanges();
    expect(counts()).toBe('1/2 states');
    expect(root.querySelector('.edge-group--uncovered')).toBeNull();
  });

  it('shows transitions only, marked as of last save while unsaved', () => {
    model();
    store.coverageView.set('transitions');
    fixture.detectChanges();
    expect(counts()).toBe('1/2 transitions');
    expect(root.querySelector('.node-group--uncovered')).toBeNull();

    store.addNode('regular', 600, 0);
    fixture.detectChanges();
    expect(counts()).toBe('1/2 transitions (as of last save)');
  });

  it('says transitions are unavailable, not zero, before a save', () => {
    model();
    store.setTransitionCoverage(null);
    store.coverageView.set('transitions');
    fixture.detectChanges();
    expect(counts()).toBe('transitions unavailable until saved');
  });

  it('turns off again', () => {
    model();
    toolbarButton().click();
    fixture.detectChanges();
    toolbarButton().click();
    fixture.detectChanges();
    expect(store.coverageView()).toBe('off');
    expect(root.querySelector('.node-group--uncovered')).toBeNull();
  });

  it('opens the test cases of an uncovered state when it is clicked', () => {
    const { a, b } = model();
    store.coverageView.set('states');
    fixture.detectChanges();
    let reveals = 0;
    fixture.componentInstance.revealTests.subscribe(() => reveals++);

    nodeGroup(a.id).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(reveals).toBe(0);

    nodeGroup(b.id).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(reveals).toBe(1);
    expect(store.selectedNode()?.id).toBe(b.id);
  });
});

describe('CanvasComponent stale tests', () => {
  it('marks a state with stale tests by a "!" chip and says so in its label', async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(CanvasComponent);
    const store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    store.updateNode(a.id, { label: 'Start' });
    const t = store.addTest(a.id, 'Path 1')!;
    fixture.detectChanges();
    const root: HTMLElement = fixture.nativeElement;
    expect(root.querySelector('.node-tests__stale')).toBeNull();

    store.setStaleTests([{ testCaseId: t.id, reasons: [{ code: 'NOT_FROM_INITIAL', message: 'm' }] }]);
    fixture.detectChanges();

    expect(root.querySelector('.node-tests__stale text')?.textContent).toBe('!');
    expect(root.querySelector('.node-tests')?.getAttribute('aria-label'))
      .toBe('Test cases on Start: 1, 1 stale. Double-click to open.');
  });
});

describe('CanvasComponent test results', () => {
  it('shows failing tests on the chips with a "✗" and passing counts in the overlay', async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(CanvasComponent);
    const store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    store.updateNode(a.id, { label: 'Start' });
    const t = store.addTest(a.id, 'Path 1')!;
    store.updateTest(a.id, t.id, { lastResult: { status: 'failed', executedAt: '' } });
    store.setTransitionCoverage({ covered: 2, total: 3, passing: 1, uncoveredTransitionIds: [] });
    store.clearDirty();
    store.coverageView.set('both');
    fixture.detectChanges();
    const root: HTMLElement = fixture.nativeElement;

    expect(root.querySelector('.node-tests__failing text')?.textContent?.trim()).toBe('✗1');
    expect(root.querySelector('.node-tests')?.getAttribute('aria-label'))
      .toBe('Test cases on Start: 1, 1 failing. Double-click to open.');
    expect(root.querySelector('.tb-coverage__counts')?.textContent?.trim())
      .toBe('1/1 states (0 passing), 2/3 transitions (1 passing)');
  });
});

describe('CanvasComponent transition routing', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let root: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    root = fixture.nativeElement;
  });

  function selectedPair() {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 400, 0);
    const e = store.addEdge(a.id, b.id);
    store.select(e.id, 'edge');
    fixture.detectChanges();
    return { a, b, e };
  }
  /** Screen position of a canvas point (inverse of the component's toCanvas). */
  const screen = (p: { x: number; y: number }) => {
    const c = fixture.componentInstance;
    const r = root.querySelector('svg')!.getBoundingClientRect();
    return { clientX: p.x * c.zoom() + c.panX() + r.left, clientY: p.y * c.zoom() + c.panY() + r.top };
  };
  const mouse = (type: string, target: EventTarget, p: { x: number; y: number }, extra = {}) =>
    target.dispatchEvent(new MouseEvent(type, { bubbles: true, button: 0, ...screen(p), ...extra }));
  const edge = (id: string) => store.edgeById(id)!;

  it('bends a transition through the dragged point and straightens it on double-click', () => {
    const { e } = selectedPair();
    const handle = root.querySelector('.edge-bend__hit')!;
    const g = fixture.componentInstance.geometries().get(e.id)!;
    mouse('mousedown', handle, g.bend!);
    mouse('mousemove', document, { x: g.bend!.x, y: g.bend!.y + 60 });
    mouse('mouseup', document, { x: g.bend!.x, y: g.bend!.y + 60 });
    fixture.detectChanges();

    expect(Math.abs(edge(e.id).curve)).toBe(120);
    const bent = fixture.componentInstance.geometries().get(e.id)!;
    expect(bent.bend!.y).toBeCloseTo(g.bend!.y + 60, 0);

    root.querySelector('.edge-bend__hit')!.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    expect(edge(e.id).curve).toBe(0);

    store.undo();
    expect(Math.abs(edge(e.id).curve)).toBe(120);
  });

  it('adds a bend point with Alt+click and from the context menu, moves it, and removes it', () => {
    const { e } = selectedPair();
    const line = root.querySelector('path.edge-hit')!;
    mouse('click', line, { x: 150, y: 80 }, { altKey: true });
    fixture.detectChanges();
    expect(edge(e.id).waypoints).toEqual([{ x: 150, y: 80 }]);

    mouse('contextmenu', line, { x: 300, y: 90 });
    fixture.componentInstance.runMenuAction('add-bend-point');
    fixture.detectChanges();
    expect(edge(e.id).waypoints).toEqual([{ x: 150, y: 80 }, { x: 300, y: 90 }]);

    const second = root.querySelectorAll('.edge-waypoint__hit')[1];
    mouse('mousedown', second, { x: 300, y: 90 });
    mouse('mousemove', document, { x: 320, y: 140 });
    mouse('mouseup', document, { x: 320, y: 140 });
    fixture.detectChanges();
    expect(edge(e.id).waypoints![1]).toEqual({ x: 320, y: 140 });

    root.querySelectorAll('.edge-waypoint__hit')[0].dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    fixture.detectChanges();
    expect(edge(e.id).waypoints).toEqual([{ x: 320, y: 140 }]);
  });

  it('drags a label off the line with a leader line, and resets it', () => {
    const { e } = selectedPair();
    expect(root.querySelector('.edge-leader')).toBeNull();
    const label = root.querySelector('.edge-label-group')!;
    const g = fixture.componentInstance.geometries().get(e.id)!;
    mouse('mousedown', label, g.label);
    mouse('mousemove', document, { x: g.label.x + 10, y: g.label.y - 50 });
    mouse('mouseup', document, { x: g.label.x + 10, y: g.label.y - 50 });
    fixture.detectChanges();

    expect(edge(e.id).labelOffset).toEqual({ dx: 10, dy: -50 });
    expect(root.querySelector('.edge-leader')).not.toBeNull();

    mouse('contextmenu', root.querySelector('path.edge-hit')!, { x: 100, y: 20 });
    fixture.componentInstance.runMenuAction('reset-label');
    fixture.detectChanges();
    expect(edge(e.id).labelOffset).toBeUndefined();
    expect(root.querySelector('.edge-leader')).toBeNull();
  });

  it('draws two self-loops on one side apart from each other', () => {
    const a = store.addNode('regular', 200, 200);
    const l1 = store.addEdge(a.id, a.id, 'right', 'right');
    const l2 = store.addEdge(a.id, a.id, 'right', 'right');
    fixture.detectChanges();
    const g1 = fixture.componentInstance.geometries().get(l1.id)!;
    const g2 = fixture.componentInstance.geometries().get(l2.id)!;
    expect(g1.d).not.toBe(g2.d);
    expect(g1.label.x).toBeGreaterThan(a.x + a.w);
    expect(Math.abs(g2.label.x - g1.label.x)).toBeGreaterThan(24);
    const paths = [...root.querySelectorAll('.edge-line')].map(p => p.getAttribute('d'));
    expect(new Set(paths).size).toBe(2);
  });

  it('switches routing from the context menu', () => {
    const { b } = selectedPair();
    const c = store.addNode('regular', 400, 300);
    const e2 = store.addEdge(b.id, c.id);
    mouse('contextmenu', root.querySelectorAll('path.edge-hit')[0], { x: 100, y: 20 });
    fixture.componentInstance.runMenuAction('routing-orthogonal');
    fixture.detectChanges();
    expect(store.edges()[0].routing).toBe('orthogonal');
    expect(store.edgeById(e2.id)!.routing).toBeUndefined();
    const d = root.querySelector('.edge-line')!.getAttribute('d')!;
    expect(d).not.toContain('Q');
    expect(d).not.toContain('C');
  });
});

describe('CanvasComponent navigation', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let root: HTMLElement;
  let c: CanvasComponent;

  beforeEach(async () => {
    localStorage.clear();
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    c = fixture.componentInstance;
    root = fixture.nativeElement;
    document.body.appendChild(root);
    fixture.detectChanges();
    // jsdom has no layout: give the canvas a size.
    const svg = root.querySelector('svg.canvas-svg') as SVGSVGElement;
    svg.getBoundingClientRect = () =>
      ({ x: 0, y: 0, left: 0, top: 0, width: 800, height: 600, right: 800, bottom: 600, toJSON: () => ({}) });
    c.canvasSize.set({ w: 800, h: 600 });
  });

  afterEach(() => root.remove());

  const key = (k: string, extra: KeyboardEventInit = {}) =>
    document.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, ...extra }));
  const focusCanvas = () => root.querySelector<HTMLElement>('.canvas-wrap')!.focus();
  const visible = (n: { x: number; y: number; w: number; h: number }) => {
    const l = n.x * c.zoom() + c.panX(), t = n.y * c.zoom() + c.panY();
    return l >= 0 && t >= 0 && l + n.w * c.zoom() <= 800 && t + n.h * c.zoom() <= 600;
  };

  it('zooms to fit the model and to the selection', () => {
    const a = store.addNode('regular', -300, -200);
    const b = store.addNode('regular', 1200, 700);
    const d = store.addNode('regular', 1600, 1000);
    key('!', { shiftKey: true, code: 'Digit1' });
    expect([a, b, d].every(n => visible(store.nodeById(n.id)!))).toBe(true);

    store.selectNodes([a.id, b.id]);
    const fitAll = c.zoom();
    key('@', { shiftKey: true, code: 'Digit2' });
    expect(visible(store.nodeById(a.id)!) && visible(store.nodeById(b.id)!)).toBe(true);
    expect(c.zoom()).toBeGreaterThan(fitAll);
  });

  it('finds states by name with Ctrl+F, steps through hits, and says when nothing matches', async () => {
    const pay = store.addNode('regular', 2000, 2000);
    store.updateNode(pay.id, { label: 'Payment' });
    const pay2 = store.addNode('regular', 0, 0);
    store.updateNode(pay2.id, { label: 'Pay later' });
    store.addNode('regular', 300, 0);
    fixture.detectChanges();
    focusCanvas();
    key('f', { ctrlKey: true });
    fixture.detectChanges();
    const input = root.querySelector<HTMLInputElement>('tm-canvas-search input')!;
    input.value = 'pay';
    input.dispatchEvent(new Event('input'));
    fixture.detectChanges();
    // Reading order: "Pay later" (top-left) first.
    expect(store.selectedNode()?.id).toBe(pay2.id);
    expect(root.querySelector('tm-canvas-search [role="status"]')!.textContent).toBe('1 of 2');

    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    fixture.detectChanges();
    expect(store.selectedNode()?.id).toBe(pay.id);
    expect(visible(store.nodeById(pay.id)!)).toBe(true);

    input.value = 'zzz';
    input.dispatchEvent(new Event('input'));
    fixture.detectChanges();
    expect(root.querySelector('tm-canvas-search [role="status"]')!.textContent).toBe('No matches');
    expect(store.selectedNode()?.id).toBe(pay.id);

    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    fixture.detectChanges();
    expect(root.querySelector('tm-canvas-search')).toBeNull();
  });

  it('shows a minimap that pans the view, and remembers when it is hidden', () => {
    store.addNode('regular', 0, 0);
    store.addNode('regular', 1600, 1200);
    fixture.detectChanges();
    const map = root.querySelector('tm-canvas-minimap svg')!;
    expect(root.querySelectorAll('.minimap__state').length).toBe(2);
    map.getBoundingClientRect = () =>
      ({ x: 0, y: 0, left: 0, top: 0, width: 180, height: 120, right: 180, bottom: 120, toJSON: () => ({}) });
    const before = [c.panX(), c.panY()];
    map.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0, clientX: 170, clientY: 110 }));
    expect([c.panX(), c.panY()]).not.toEqual(before);

    c.toggleMinimap();
    fixture.detectChanges();
    expect(root.querySelector('tm-canvas-minimap')).toBeNull();
    expect(localStorage.getItem('tm:editor-minimap')).toBe('false');
  });

  it('collapses a group into a box that transitions attach to, and expands it again', () => {
    const inside = [0, 1, 2, 3].map(i => store.addNode('regular', i * 160, 0));
    const outside = store.addNode('regular', 0, 400);
    const inner = store.addEdge(inside[0].id, inside[1].id);
    const cross = store.addEdge(outside.id, inside[2].id);
    store.selectNodes(inside.map(n => n.id));
    const g = store.groupSelection('Auth')!;
    fixture.detectChanges();

    store.collapseGroup(g.id);
    fixture.detectChanges();
    expect(root.querySelectorAll('g.node-group').length).toBe(1);
    expect(root.querySelector('.group-box__count')!.textContent!.trim()).toBe('4 states');
    expect(c.geometries().has(inner.id)).toBe(false);
    const box = store.groupById(g.id)!;
    expect(c.geometries().get(cross.id)!.tgt.y).toBeLessThanOrEqual(box.y + 56 + 3);

    // Saved with the layout.
    expect(store.toPersisted('m').groups![0]).toMatchObject({ collapsed: true });

    // Revealing a hidden state (search, validation) expands the group.
    c.revealElement(inside[3].id);
    fixture.detectChanges();
    expect(store.groupById(g.id)!.collapsed).toBeUndefined();
    expect(root.querySelectorAll('g.node-group').length).toBe(5);
    expect(store.nodes().map(n => [n.x, n.y])).toEqual([[0, 0], [160, 0], [320, 0], [480, 0], [0, 400]]);
  });

  it('snaps a dragged state to its neighbour with a guide, and not while Alt is held', () => {
    const a = store.addNode('regular', 0, 0);
    store.addNode('regular', 400, 200);
    fixture.detectChanges();
    const el = () => root.querySelector<SVGGElement>(`[data-node-id="${a.id}"]`)!;
    const at = (x: number, y: number, extra = {}) =>
      ({ bubbles: true, button: 0, clientX: x + c.panX(), clientY: y + c.panY(), ...extra });

    el().dispatchEvent(new MouseEvent('mousedown', at(10, 10)));
    document.dispatchEvent(new MouseEvent('mousemove', at(13, 207)));
    fixture.detectChanges();
    expect(store.nodeById(a.id)!.y).toBe(200);
    expect(root.querySelectorAll('.snap-guide').length).toBeGreaterThan(0);
    document.dispatchEvent(new MouseEvent('mouseup', at(13, 207)));
    fixture.detectChanges();
    expect(root.querySelectorAll('.snap-guide').length).toBe(0);

    el().dispatchEvent(new MouseEvent('mousedown', at(10, 210)));
    document.dispatchEvent(new MouseEvent('mousemove', at(13, 215, { altKey: true })));
    expect(store.nodeById(a.id)!.y).toBe(205);
    document.dispatchEvent(new MouseEvent('mouseup', at(13, 215)));
  });

  it('moves between states with Tab in reading order and between transitions with Ctrl+Tab', async () => {
    const b = store.addNode('regular', 400, 0);
    const a = store.addNode('regular', 0, 0);
    const d = store.addNode('regular', 0, 300);
    const e1 = store.addEdge(a.id, b.id);
    fixture.detectChanges();
    focusCanvas();
    key('Tab');
    expect(store.selectedNode()?.id).toBe(a.id);
    key('Tab');
    expect(store.selectedNode()?.id).toBe(b.id);
    key('Tab', { shiftKey: true });
    expect(store.selectedNode()?.id).toBe(a.id);
    await new Promise(r => setTimeout(r));
    expect(document.activeElement?.getAttribute('data-node-id')).toBe(a.id);
    key('Tab'); key('Tab');
    expect(store.selectedNode()?.id).toBe(d.id);
    key('Tab', { ctrlKey: true });
    expect(store.selectedEdge()?.id).toBe(e1.id);
  });

  it('moves the selection with arrows as one undo step, renames with F2, zooms with + and -', () => {
    const a = store.addNode('regular', 0, 0);
    store.select(a.id, 'node');
    fixture.detectChanges();
    focusCanvas();
    key('ArrowRight');
    key('ArrowRight');
    key('ArrowDown', { shiftKey: true });
    expect([store.nodeById(a.id)!.x, store.nodeById(a.id)!.y]).toEqual([16, 80]);
    store.undo();
    expect([store.nodeById(a.id)!.x, store.nodeById(a.id)!.y]).toEqual([0, 0]);

    const z = c.zoom();
    key('+');
    expect(c.zoom()).toBeGreaterThan(z);
    key('-');
    expect(c.zoom()).toBeCloseTo(z);

    focusCanvas();
    key('F2');
    fixture.detectChanges();
    expect(root.querySelector('textarea.node-edit-input')).not.toBeNull();
  });

  it('lists every shortcut in the help on ?', () => {
    key('?');
    fixture.detectChanges();
    const rows = root.querySelectorAll('.shortcuts tr');
    expect(rows.length).toBe(c.helpRows.length);
    expect(root.querySelector('.shortcuts')!.textContent).toContain('Zoom to fit the model');
    key('Escape');
    fixture.detectChanges();
    expect(root.querySelector('.shortcuts')).toBeNull();
  });

  it('leaves Tab and arrows alone when the canvas does not have focus', () => {
    const a = store.addNode('regular', 0, 0);
    store.select(a.id, 'node');
    (document.activeElement as HTMLElement | null)?.blur();
    key('ArrowRight');
    expect(store.nodeById(a.id)!.x).toBe(0);
  });
});

describe('CanvasComponent path highlight', () => {
  it('marks the path, dims the rest, numbers steps, announces it, and clears on Esc', async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(CanvasComponent);
    const store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    store.updateNode(a.id, { label: 'Start' });
    const b = store.addNode('regular', 300, 0);
    const off = store.addNode('regular', 0, 300);
    const e = store.addEdge(a.id, b.id);
    store.addEdge(a.id, off.id);
    const t = store.addTest(a.id, 'Login works')!;
    store.updateTest(a.id, t.id, {
      path: [
        { targetId: a.id, kind: 'state' },
        { targetId: e.id, kind: 'transition', stepOrder: 1 },
        { targetId: b.id, kind: 'state', stepOrder: 1 },
      ],
    });
    store.showPath(t.id);
    fixture.detectChanges();
    const root: HTMLElement = fixture.nativeElement;

    const node = (id: string) => root.querySelector(`[data-node-id="${id}"]`)!;
    expect(node(b.id).classList).toContain('node-group--on-path');
    expect(node(off.id).classList).toContain('node-group--dimmed');
    expect(node(b.id).querySelector('.path-badge text')!.textContent).toBe('1');
    expect(root.querySelectorAll('.edge-group--on-path').length).toBe(1);
    expect(root.querySelectorAll('.edge-group--dimmed').length).toBe(1);
    expect(root.querySelector('.path-banner')!.textContent).toContain('Path of “Login works” — 1 step');
    expect(node(b.id).getAttribute('aria-label')).toContain('path step 1');
    const hit = [...root.querySelectorAll('path.edge-hit')].find(p => p.getAttribute('aria-label')?.includes('path step 1'));
    expect(hit).toBeTruthy();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    fixture.detectChanges();
    expect(root.querySelector('.path-banner')).toBeNull();
    expect(root.querySelector('.node-group--dimmed')).toBeNull();
  });
});

describe('CanvasComponent present mode (read-only)', () => {
  it('allows no edit but keeps selection, zoom and search', async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(CanvasComponent);
    const store = TestBed.inject(ModelEditorStore);
    fixture.componentRef.setInput('readonly', true);
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 300, 0);
    const e = store.addEdge(a.id, b.id);
    fixture.detectChanges();
    const root: HTMLElement = fixture.nativeElement;
    document.body.appendChild(root);
    const c = fixture.componentInstance;
    const node = root.querySelector<SVGGElement>(`[data-node-id="${a.id}"]`)!;

    // Editing controls are gone.
    expect(getComputedStyle(root.querySelector('.quickbar')!).display).toBe('none');
    // Dragging a state pans instead of moving it.
    const pan = c.panX();
    node.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0, clientX: 50, clientY: 20 }));
    document.dispatchEvent(new MouseEvent('mousemove', { bubbles: true, clientX: 150, clientY: 20 }));
    document.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, clientX: 150, clientY: 20 }));
    expect(store.nodeById(a.id)!.x).toBe(0);
    expect(c.panX()).toBe(pan + 100);
    // …but a click still selects.
    expect(store.selectedNode()?.id).toBe(a.id);

    // No delete, rename, context menu, connector dots or transition handles.
    node.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'F2', bubbles: true }));
    node.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    node.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }));
    node.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
    fixture.detectChanges();
    expect(store.nodes().length).toBe(2);
    expect(root.querySelector('textarea.node-edit-input')).toBeNull();
    expect(c.contextMenu()).toBeNull();
    expect(root.querySelector('.connector-dot')).toBeNull();
    store.select(e.id, 'edge');
    fixture.detectChanges();
    expect(root.querySelector('.edge-handle-hit, .edge-bend__hit')).toBeNull();

    // Navigation still works.
    const z = c.zoom();
    c.zoomIn();
    expect(c.zoom()).toBeGreaterThan(z);
    c.openSearch();
    fixture.detectChanges();
    expect(root.querySelector('tm-canvas-search')).not.toBeNull();
    root.remove();
  });
});

describe('CanvasComponent simulation marks', () => {
  it('marks the current state with text and the enabled and blocked transitions', async () => {
    await TestBed.configureTestingModule({ imports: [CanvasComponent], providers: [ModelEditorStore] }).compileComponents();
    const fixture = TestBed.createComponent(CanvasComponent);
    const store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    const b = store.addNode('regular', 300, 0);
    const go = store.addEdge(a.id, b.id);
    const no = store.addEdge(a.id, a.id);
    store.simulation.set({ currentId: a.id, enabled: new Set([go.id]), blocked: new Set([no.id]) });
    fixture.detectChanges();
    const root: HTMLElement = fixture.nativeElement;
    expect(root.querySelector(`[data-node-id="${a.id}"] .sim-badge text`)!.textContent).toBe('▶ now');
    expect(root.querySelectorAll('.edge-group--sim-enabled').length).toBe(1);
    expect(root.querySelectorAll('.edge-group--sim-blocked').length).toBe(1);
  });
});
