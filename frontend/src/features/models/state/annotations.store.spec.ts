import { TestBed } from '@angular/core/testing';
import { ANNOTATION_MAX_TEXT, ANNOTATION_SIZE, ModelEditorStore } from './model-editor.store';

describe('ModelEditorStore annotations', () => {
  let store: ModelEditorStore;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [ModelEditorStore] });
    store = TestBed.inject(ModelEditorStore);
  });

  const note = (id: string) => store.annotationById(id)!;

  it('adds an empty note or text box at its default size, as an undo step', () => {
    const n = store.addAnnotation('note', 40, 60);
    const t = store.addAnnotation('text', 300, 60);
    expect(n).toMatchObject({ kind: 'note', x: 40, y: 60, text: '', ...ANNOTATION_SIZE.note });
    expect(t.kind).toBe('text');
    store.undo();
    expect(store.annotations().map(a => a.id)).toEqual([n.id]);
  });

  it('keeps notes out of the graph', () => {
    store.addAnnotation('note', 0, 0);
    expect(store.nodes()).toEqual([]);
    expect(store.edges()).toEqual([]);
    expect(store.issues().some(i => i.code !== 'NO_INITIAL_STATE')).toBe(false);
    expect(store.searchElements('')).toEqual([]);
  });

  it('cuts text at the limit and says so', () => {
    const n = store.addAnnotation('note', 0, 0);
    expect(store.setAnnotationText(n.id, 'x'.repeat(3000))).toBe(true);
    expect(note(n.id).text.length).toBe(ANNOTATION_MAX_TEXT);
    expect(store.setAnnotationText(n.id, 'short')).toBe(false);
  });

  it('grows a note to fit its text, and never shrinks it below what the user set', () => {
    const n = store.addAnnotation('note', 0, 0);
    store.setAnnotationText(n.id, Array.from({ length: 12 }, (_, i) => `line ${i}`).join('\n'));
    expect(note(n.id).h).toBeGreaterThan(ANNOTATION_SIZE.note.h);
    store.setAnnotationText(n.id, 'one line');
    expect(note(n.id).h).toBeGreaterThan(ANNOTATION_SIZE.note.h);
  });

  it('wraps text to a narrower width and grows taller', () => {
    const n = store.addAnnotation('note', 0, 0);
    store.setAnnotationText(n.id, 'Remember to check the payment provider timeout settings first');
    const before = { lines: store.annotationLines(note(n.id)).length, h: note(n.id).h };
    store.resizeAnnotation(n.id, 'e', { x: 0, y: 0, w: note(n.id).w, h: note(n.id).h }, -90, 0);
    expect(store.annotationLines(note(n.id)).length).toBeGreaterThan(before.lines);
    expect(note(n.id).h).toBeGreaterThanOrEqual(before.h);
  });

  it('removes an annotation whose editor closes empty, and drops trailing blank lines', () => {
    const a = store.addAnnotation('note', 0, 0);
    const b = store.addAnnotation('text', 0, 0);
    store.commitAnnotationText(a.id, '   \n ');
    store.commitAnnotationText(b.id, 'Hello\nworld\n\n');
    expect(store.annotationById(a.id)).toBeUndefined();
    expect(note(b.id).text).toBe('Hello\nworld');
  });

  it('selects, aligns and deletes annotations with states', () => {
    const s = store.addNode('regular', 0, 100);
    const n = store.addAnnotation('note', 300, 0);
    expect(store.annotationsInRect(250, -10, 600, 50)).toEqual([n.id]);
    store.selectItems([s.id], [n.id]);
    store.alignSelection('top');
    expect(note(n.id).y).toBe(0);
    expect(store.nodeById(s.id)!.y).toBe(0);

    store.deleteSelection();
    expect(store.annotations()).toEqual([]);
    expect(store.nodes()).toEqual([]);
    store.undo();
    expect(store.annotations().length).toBe(1);
  });

  it('styles notes and text boxes with the keys they support', () => {
    const n = store.addAnnotation('note', 0, 0);
    const t = store.addAnnotation('text', 0, 0);
    store.setStyle([n.id, t.id], 'fill', '#dbeafe');
    store.setStyle([n.id, t.id], 'bold', true);
    expect(note(n.id).style).toEqual({ fill: '#dbeafe', bold: true });
    expect(note(t.id).style).toEqual({ bold: true });
  });

  it('is kept by loadFrom and toPersisted, and cleared by reset', () => {
    const n = store.addAnnotation('note', 0, 0);
    store.setAnnotationText(n.id, 'hi');
    const saved = store.toPersisted('m');
    store.reset();
    expect(store.annotations()).toEqual([]);
    store.loadFrom({ ...saved, scenarioDesc: '' });
    expect(store.annotations()[0].text).toBe('hi');
  });
});
