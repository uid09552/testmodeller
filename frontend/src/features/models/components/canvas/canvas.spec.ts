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
