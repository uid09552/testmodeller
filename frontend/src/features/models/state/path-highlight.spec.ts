import { TestBed } from '@angular/core/testing';
import { TestCase } from '../../../core/api/api.types';
import { testFromApi } from './model-mapping';
import { ModelEditorStore } from './model-editor.store';

const M = 'm1';

describe('test path highlight', () => {
  let store: ModelEditorStore;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [ModelEditorStore] });
    store = TestBed.inject(ModelEditorStore);
  });

  it('keeps the assignments in this model, with step numbers', () => {
    const tc: TestCase = {
      id: 't', version: 1, featureId: 'f', name: 'Path 1', steps: [], tags: [],
      assignments: [
        { modelId: M, stateId: 's0', testCaseId: 't' },
        { modelId: M, transitionId: 'e1', stepOrder: 1, testCaseId: 't' },
        { modelId: 'other', stateId: 'x', testCaseId: 't' },
      ],
    };
    expect(testFromApi(tc, 1, M).path).toEqual([
      { targetId: 's0', kind: 'state' },
      { targetId: 'e1', kind: 'transition', stepOrder: 1 },
    ]);
    expect(testFromApi(tc, 1).path).toBeUndefined();
  });

  it('numbers every step, a repeated transition with both numbers', () => {
    const a = store.addNode('initial', 0, 0);
    const b = store.addNode('regular', 300, 0);
    const loop = store.addEdge(b.id, b.id);
    const t = store.addTest(a.id, 'Generated 1')!;
    store.updateTest(a.id, t.id, {
      path: [
        { targetId: a.id, kind: 'state' },
        { targetId: 'e0', kind: 'transition', stepOrder: 1 },
        { targetId: b.id, kind: 'state', stepOrder: 1 },
        { targetId: loop.id, kind: 'transition', stepOrder: 2 },
        { targetId: loop.id, kind: 'transition', stepOrder: 3 },
      ],
    });

    expect(store.showPath(t.id)).toBe(true);
    expect(store.pathSteps(loop.id)).toEqual([2, 3]);
    expect(store.pathSteps(a.id)).toEqual([]);
    expect(store.pathSteps('elsewhere')).toBeNull();
    expect(store.highlight()?.count).toBe(3);
    // The first step is brought into view.
    expect(store.revealRequest()?.id).toBe('e0');

    store.clearHighlight();
    expect(store.pathSteps(loop.id)).toBeNull();
  });

  it('highlights just the state of a test that has no stored path', () => {
    const a = store.addNode('regular', 0, 0);
    const t = store.addTest(a.id, 'New')!;
    store.showPath(t.id);
    expect(store.pathSteps(a.id)).toEqual([]);
    expect(store.highlight()?.count).toBe(0);
  });
});
