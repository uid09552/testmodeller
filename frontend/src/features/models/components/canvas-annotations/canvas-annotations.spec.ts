import { TestBed } from '@angular/core/testing';
import { CanvasComponent } from '../canvas/canvas';
import { ANNOTATION_SIZE, ModelEditorStore } from '../../state/model-editor.store';

const mouse = (type: string, x: number, y: number, init: MouseEventInit = {}) =>
  new MouseEvent(type, { button: 0, clientX: x, clientY: y, bubbles: true, ...init });

describe('CanvasAnnotationsComponent', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let root: HTMLElement;

  async function setup(readonly = false) {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    fixture.componentRef.setInput('readonly', readonly);
    root = fixture.nativeElement;
    document.body.appendChild(root);
    fixture.detectChanges();
  }
  afterEach(() => root?.remove());

  const noteEl = (id: string) => root.querySelector<SVGGElement>(`[data-annotation-id="${id}"]`)!;
  const lines = (id: string) => [...noteEl(id).querySelectorAll('tspan')].map(t => t.textContent);
  const tick = () => new Promise(r => setTimeout(r));

  it('draws a note with its text on several lines and a text box without an outline', async () => {
    await setup();
    const n = store.addAnnotation('note', 0, 0);
    const t = store.addAnnotation('text', 0, 200);
    store.setAnnotationText(n.id, 'First line\nSecond line');
    store.setAnnotationText(t.id, 'Title');
    fixture.detectChanges();
    expect(noteEl(n.id).querySelector('.note-bg')).not.toBeNull();
    expect(lines(n.id)).toEqual(['First line', 'Second line']);
    expect(noteEl(t.id).querySelector('.note-bg')).toBeNull();
    expect(noteEl(t.id).getAttribute('aria-label')).toBe('Text: Title');
  });

  it('wraps text to a narrower width when resized by a handle, and the note grows taller', async () => {
    await setup();
    const n = store.addAnnotation('note', 0, 0);
    store.setAnnotationText(n.id, 'Remember to check the payment provider timeout settings');
    store.select(n.id, 'annotation');
    fixture.detectChanges();
    const before = { lines: lines(n.id).length, h: store.annotationById(n.id)!.h };

    noteEl(n.id).querySelector('[data-handle="e"]')!.dispatchEvent(mouse('mousedown', 200, 50));
    document.dispatchEvent(mouse('mousemove', 110, 50));
    document.dispatchEvent(mouse('mouseup', 110, 50));
    fixture.detectChanges();

    expect(store.annotationById(n.id)!.w).toBe(ANNOTATION_SIZE.note.w - 90);
    expect(lines(n.id).length).toBeGreaterThan(before.lines);
    expect(store.annotationById(n.id)!.h).toBeGreaterThanOrEqual(before.h);
    store.undo();
    expect(store.annotationById(n.id)!.w).toBe(ANNOTATION_SIZE.note.w);
  });

  it('edits in place: Shift+Enter keeps typing, Enter commits, Escape restores', async () => {
    await setup();
    const n = store.addAnnotation('note', 0, 0);
    store.setAnnotationText(n.id, 'Old');
    fixture.detectChanges();
    noteEl(n.id).dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    fixture.detectChanges();
    await tick();
    const ta = noteEl(n.id).querySelector('textarea')!;
    expect(ta.value).toBe('Old');
    expect(document.activeElement).toBe(ta);

    const shiftEnter = new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true, cancelable: true });
    ta.dispatchEvent(shiftEnter);
    expect(shiftEnter.defaultPrevented).toBe(false);
    ta.value = 'Two\nlines';
    ta.dispatchEvent(new Event('input'));
    ta.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', cancelable: true }));
    fixture.detectChanges();
    expect(store.annotationById(n.id)!.text).toBe('Two\nlines');
    expect(noteEl(n.id).querySelector('textarea')).toBeNull();

    // One undo step for the whole edit.
    store.undo();
    expect(store.annotationById(n.id)!.text).toBe('Old');

    fixture.componentInstance.editAnnotation(n.id);
    fixture.detectChanges();
    await tick();
    const again = noteEl(n.id).querySelector('textarea')!;
    again.value = 'Discard me';
    again.dispatchEvent(new Event('input'));
    again.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', cancelable: true }));
    expect(store.annotationById(n.id)!.text).toBe('Old');
  });

  it('opens the editor with F2 and says when pasted text was cut', async () => {
    await setup();
    const n = store.addAnnotation('note', 0, 0);
    store.select(n.id, 'annotation');
    fixture.detectChanges();
    root.querySelector<HTMLElement>('.canvas-wrap')!.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'F2', bubbles: true }));
    fixture.detectChanges();
    await tick();
    const ta = noteEl(n.id).querySelector('textarea')!;
    expect(ta).not.toBeNull();
    ta.value = 'x'.repeat(3000);
    ta.dispatchEvent(new Event('input'));
    fixture.detectChanges();
    expect(ta.value.length).toBe(2000);
    expect(noteEl(n.id).querySelector('[role="status"]')!.textContent).toContain('2000');
  });

  it('removes a new annotation whose editor closes empty', async () => {
    await setup();
    const n = store.addAnnotation('note', 0, 0);
    fixture.componentInstance.editAnnotation(n.id);
    fixture.detectChanges();
    await tick();
    noteEl(n.id).querySelector('textarea')!.dispatchEvent(new Event('blur'));
    expect(store.annotations()).toEqual([]);
  });

  it('moves with selected states as one undo step', async () => {
    await setup();
    const s = store.addNode('regular', 0, 200);
    const n = store.addAnnotation('note', 0, 0);
    store.setAnnotationText(n.id, 'Hi');
    store.selectItems([s.id], [n.id]);
    fixture.detectChanges();
    fixture.componentInstance.snapOn.set(false);

    noteEl(n.id).dispatchEvent(mouse('mousedown', 50, 50));
    document.dispatchEvent(mouse('mousemove', 80, 70));
    document.dispatchEvent(mouse('mousemove', 150, 90));
    document.dispatchEvent(mouse('mouseup', 150, 90));
    expect(store.annotationById(n.id)).toMatchObject({ x: 100, y: 40 });
    expect(store.nodeById(s.id)).toMatchObject({ x: 100, y: 240 });

    store.undo();
    expect(store.annotationById(n.id)).toMatchObject({ x: 0, y: 0 });
    expect(store.nodeById(s.id)).toMatchObject({ x: 0, y: 200 });
  });

  it('snaps to a nearby state while dragged, with a guide', async () => {
    await setup();
    store.addNode('regular', 300, 0);
    const n = store.addAnnotation('note', 0, 100);
    fixture.detectChanges();
    fixture.componentInstance.snapOn.set(true);
    noteEl(n.id).dispatchEvent(mouse('mousedown', 50, 150));
    document.dispatchEvent(mouse('mousemove', 50, 75));
    expect(store.annotationById(n.id)!.y).toBe(24);
    expect(fixture.componentInstance.guides().length).toBeGreaterThan(0);
    document.dispatchEvent(mouse('mouseup', 50, 75));
  });

  it('is shown in present mode, but cannot be edited, moved or resized', async () => {
    await setup(true);
    const n = store.addAnnotation('note', 0, 0);
    store.setAnnotationText(n.id, 'Read me');
    store.select(n.id, 'annotation');
    fixture.detectChanges();
    expect(lines(n.id)).toEqual(['Read me']);
    expect(noteEl(n.id).querySelector('[data-handle]')).toBeNull();

    noteEl(n.id).dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    fixture.detectChanges();
    expect(noteEl(n.id).querySelector('textarea')).toBeNull();

    noteEl(n.id).dispatchEvent(mouse('mousedown', 50, 50));
    document.dispatchEvent(mouse('mousemove', 150, 90));
    document.dispatchEvent(mouse('mouseup', 150, 90));
    expect(store.annotationById(n.id)).toMatchObject({ x: 0, y: 0 });
  });
});

describe('Annotation palette tools', () => {
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
    document.body.appendChild(root);
    fixture.detectChanges();
    // jsdom lays nothing out: give the canvas a size, so drops land inside it.
    const svg = root.querySelector('svg.canvas-svg')!;
    svg.getBoundingClientRect = () => ({ left: 0, top: 0, right: 800, bottom: 600, width: 800, height: 600 } as DOMRect);
  });
  afterEach(() => root.remove());

  const drop = (x: number, y: number) =>
    Object.assign(new MouseEvent('drop', { clientX: x, clientY: y }), { dataTransfer: null }) as unknown as DragEvent;

  it('offers a note and a text tool instead of the border colour row', () => {
    const tools = [...root.querySelectorAll('.quickbar .qb-tool')].map(b => b.getAttribute('aria-label'));
    expect(tools).toEqual(expect.arrayContaining(['Note', 'Text']));
    expect(root.querySelector('.qb-colors')).toBeNull();
  });

  it('drops a note onto the canvas with its editor open', async () => {
    const c = fixture.componentInstance;
    c.onPaletteDragStart({ dataTransfer: null } as unknown as DragEvent, 'note');
    c.onCanvasDrop(drop(240, 140));
    fixture.detectChanges();
    await new Promise(r => setTimeout(r));
    const [a] = store.annotations();
    expect(a.kind).toBe('note');
    // Centred on the drop point (pan 40, 40; zoom 1).
    expect([a.x + a.w / 2, a.y + a.h / 2]).toEqual([200, 100]);
    expect(store.selectedAnnotation()?.id).toBe(a.id);
    expect(document.activeElement?.tagName.toLowerCase()).toBe('textarea');
    expect(store.nodes()).toEqual([]);
  });

  it('drops a text box too', () => {
    const c = fixture.componentInstance;
    c.onPaletteDragStart({ dataTransfer: null } as unknown as DragEvent, 'text');
    c.onCanvasDrop(drop(240, 140));
    expect(store.annotations()[0].kind).toBe('text');
  });

  it('creates no transition, and no state, when a transition is dropped on a note', () => {
    const s = store.addNode('regular', 0, 0);
    const n = store.addAnnotation('note', 300, 0);
    store.setAnnotationText(n.id, 'Here');
    fixture.detectChanges();
    const c = fixture.componentInstance;

    c.onConnectorMouseDown(new MouseEvent('mousedown', { button: 0 }), s, 'right');
    // (380, 80) in canvas coordinates is inside the note.
    document.dispatchEvent(new MouseEvent('mouseup', { clientX: 420, clientY: 120, bubbles: true }));
    expect(store.edges()).toEqual([]);
    expect(store.nodes().length).toBe(1);

    // Released on empty canvas instead, the same drag does create a state.
    c.onConnectorMouseDown(new MouseEvent('mousedown', { button: 0 }), s, 'right');
    document.dispatchEvent(new MouseEvent('mouseup', { clientX: 440, clientY: 440, bubbles: true }));
    expect(store.edges().length).toBe(1);
  });
});
