import { TestBed } from '@angular/core/testing';
import { ModelEditorStore } from './model-editor.store';

describe('ModelEditorStore stale tests', () => {
  let store: ModelEditorStore;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [ModelEditorStore] });
    store = TestBed.inject(ModelEditorStore);
  });

  function withTests() {
    const a = store.addNode('initial');
    const b = store.addNode('final');
    store.addEdge(a.id, b.id);
    const t1 = store.addTest(a.id, 'Path 1')!;
    const t2 = store.addTest(a.id, 'Path 2')!;
    return { a, b, t1, t2 };
  }

  it('holds the reasons per test case and counts them per state and model', () => {
    const { a, b, t1 } = withTests();
    store.setStaleTests([{
      testCaseId: t1.id,
      reasons: [{ code: 'STEP_UNASSIGNED', stepOrder: 2, message: 'step 2 lost its transition' }],
    }]);

    expect(store.staleReasons(t1.id).map(r => r.code)).toEqual(['STEP_UNASSIGNED']);
    expect(store.staleCountOn(store.nodeById(a.id)!)).toBe(1);
    expect(store.staleCountOn(store.nodeById(b.id)!)).toBe(0);
    expect(store.staleCount()).toBe(1);
  });

  it('lists stale tests as validation warnings with their reasons', () => {
    const { a, t1 } = withTests();
    const before = store.warningCount();
    store.setStaleTests([{
      testCaseId: t1.id,
      reasons: [{ code: 'GUARD_UNSATISFIABLE', stepOrder: 1, message: 'guard `x` of step 1 is false' }],
    }]);

    const issue = store.issues().find(i => i.code === 'STALE_TEST')!;
    expect(issue.severity).toBe('warning');
    expect(issue.elementId).toBe(a.id);
    expect(issue.message).toContain('Path 1');
    expect(issue.message).toContain('guard `x` of step 1 is false');
    expect(store.warningCount()).toBe(before + 1);
  });

  it('forgets an answer when the result is replaced or another model opens', () => {
    const { t1 } = withTests();
    store.setStaleTests([{ testCaseId: t1.id, reasons: [{ code: 'NOT_FROM_INITIAL', message: 'm' }] }]);
    store.setStaleTests([]);
    expect(store.staleCount()).toBe(0);

    store.setStaleTests([{ testCaseId: t1.id, reasons: [{ code: 'NOT_FROM_INITIAL', message: 'm' }] }]);
    store.reset();
    expect(store.staleTests()).toEqual({});
  });
});
