import { TestBed } from '@angular/core/testing';
import { ModelEditorStore, withStyle } from './model-editor.store';

describe('ModelEditorStore element styling', () => {
  let store: ModelEditorStore;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [ModelEditorStore] });
    store = TestBed.inject(ModelEditorStore);
  });

  function pair() {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 400, 0);
    const e = store.addEdge(a.id, b.id);
    return { a, b, e };
  }
  const node = (id: string) => store.nodeById(id)!;
  const edge = (id: string) => store.edgeById(id)!;

  it('sets a style key on one element', () => {
    const { e } = pair();
    store.setStyle([e.id], 'dash', 'dashed');
    store.setStyle([e.id], 'width', 3);
    expect(edge(e.id).style).toEqual({ dash: 'dashed', width: 3 });
  });

  it('removes the key on "Default", and the style when nothing is left', () => {
    const { a } = pair();
    store.setStyle([a.id], 'stroke', '#ef4444');
    store.setStyle([a.id], 'bold', true);
    store.setStyle([a.id], 'stroke', null);
    expect(node(a.id).style).toEqual({ bold: true });
    store.setStyle([a.id], 'bold', false);
    expect(node(a.id).style).toBeUndefined();
  });

  it('styles a mixed selection, skipping keys an element does not support', () => {
    const { a, b, e } = pair();
    store.setStyle([a.id, b.id, e.id], 'fill', '#123456');
    expect(node(a.id).style?.fill).toBe('#123456');
    expect(node(b.id).style?.fill).toBe('#123456');
    expect(edge(e.id).style).toBeUndefined();

    store.setStyle([a.id, e.id], 'arrow', 'open');
    expect(node(a.id).style?.arrow).toBeUndefined();
    expect(edge(e.id).style?.arrow).toBe('open');
  });

  it('undoes a bulk change in one step', () => {
    const { a, b, e } = pair();
    store.selection.set([
      { id: a.id, type: 'node' }, { id: b.id, type: 'node' }, { id: e.id, type: 'edge' },
    ]);
    store.styleSelection('dash', 'dotted');
    expect([node(a.id), node(b.id), edge(e.id)].map(x => x.style?.dash))
      .toEqual(['dotted', 'dotted', 'dotted']);
    store.undo();
    expect([node(a.id), node(b.id), edge(e.id)].map(x => x.style)).toEqual([undefined, undefined, undefined]);
  });

  it('records no undo step when nothing supports the key', () => {
    const { e } = pair();
    const before = store.canUndo();
    store.undo(); // drop the addEdge step
    store.redo();
    store.setStyle([e.id], 'fill', '#000000');
    expect(edge(e.id).style).toBeUndefined();
    expect(store.canUndo()).toBe(before);
  });

  it('colours the border of the selected states through the style', () => {
    const { a, b } = pair();
    store.selectNodes([a.id, b.id]);
    store.colorSelection('#22c55e');
    expect([node(a.id).style?.stroke, node(b.id).style?.stroke]).toEqual(['#22c55e', '#22c55e']);
  });

  it('withStyle never mutates its input', () => {
    const s = { stroke: '#000000' };
    expect(withStyle(s, 'dash', 'dashed')).toEqual({ stroke: '#000000', dash: 'dashed' });
    expect(s).toEqual({ stroke: '#000000' });
  });
});
