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
