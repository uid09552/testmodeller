import { TestBed } from '@angular/core/testing';
import { ModelEditorStore } from './model-editor.store';

describe('ModelEditorStore transition routing', () => {
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
  const edge = (id: string) => store.edgeById(id)!;

  it('fans out parallel transitions between the same dots', () => {
    const { a, b, e } = pair();
    const second = store.addEdge(a.id, b.id);
    const third = store.addEdge(a.id, b.id);
    expect([edge(e.id).curve, second.curve, third.curve]).toEqual([0, 46, -46]);
  });

  it('bends, straightens and undoes', () => {
    const { e } = pair();
    store.setEdgeCurve(e.id, 60);
    store.addWaypoint(e.id, 0, { x: 200, y: 120 });
    expect(edge(e.id).curve).toBe(60);
    store.straighten(e.id);
    expect([edge(e.id).curve, edge(e.id).waypoints]).toEqual([0, undefined]);
    store.undo();
    expect(edge(e.id).waypoints).toEqual([{ x: 200, y: 120 }]);
    store.undo();
    store.undo();
    expect(edge(e.id).curve).toBe(0);
    store.redo();
    expect(edge(e.id).curve).toBe(60);
  });

  it('adds waypoints in path order, moves and removes them', () => {
    const { e } = pair();
    store.addWaypoint(e.id, 0, { x: 300, y: 100 });
    store.addWaypoint(e.id, 0, { x: 100, y: 100.4 });
    expect(edge(e.id).waypoints).toEqual([{ x: 100, y: 100 }, { x: 300, y: 100 }]);
    store.moveWaypointLive(e.id, 1, { x: 310, y: 90 });
    expect(edge(e.id).waypoints![1]).toEqual({ x: 310, y: 90 });
    store.removeWaypoint(e.id, 0);
    store.removeWaypoint(e.id, 0);
    expect(edge(e.id).waypoints).toBeUndefined();
  });

  it('switches routing style and resets a label offset', () => {
    const { e } = pair();
    store.setRouting(e.id, 'orthogonal');
    expect(edge(e.id).routing).toBe('orthogonal');
    store.setRouting(e.id, 'curved');
    expect(edge(e.id).routing).toBeUndefined();
    store.setLabelOffsetLive(e.id, 10.4, -30);
    expect(edge(e.id).labelOffset).toEqual({ dx: 10, dy: -30 });
    store.resetLabelOffset(e.id);
    expect(edge(e.id).labelOffset).toBeUndefined();
  });

  it('moves the waypoints of transitions inside a group with the group, and only those', () => {
    const { a, b, e } = pair();
    const outside = store.addNode('regular', 900, 0);
    const out = store.addEdge(b.id, outside.id);
    store.addWaypoint(e.id, 0, { x: 200, y: 100 });
    store.addWaypoint(out.id, 0, { x: 700, y: 100 });
    store.selectNodes([a.id, b.id]);
    const g = store.groupSelection('G')!;

    store.moveGroup(g.id, 10, 20);

    expect(edge(e.id).waypoints).toEqual([{ x: 210, y: 120 }]);
    expect(edge(out.id).waypoints).toEqual([{ x: 700, y: 100 }]);
  });
});
