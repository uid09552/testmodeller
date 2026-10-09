import { TestBed } from '@angular/core/testing';
import { coverageLabel, ModelEditorStore, stateCoverage } from './model-editor.store';

describe('stateCoverage', () => {
  it('is 0/0 for an empty model', () => {
    const c = stateCoverage([]);
    expect([c.covered, c.total, c.uncovered.size]).toEqual([0, 0, 0]);
  });

  it('counts every state with a test case as covered', () => {
    const store = new ModelEditorStore();
    const a = store.addNode('initial');
    const b = store.addNode('final');
    store.addTest(a.id);
    store.addTest(b.id);
    const c = stateCoverage(store.nodes());
    expect([c.covered, c.total, c.uncovered.size]).toEqual([2, 2, 0]);
  });

  it('lists every state as uncovered when none has a test', () => {
    const store = new ModelEditorStore();
    const a = store.addNode('initial');
    const b = store.addNode('final');
    const c = stateCoverage(store.nodes());
    expect([c.covered, c.total]).toEqual([0, 2]);
    expect([...c.uncovered]).toEqual([a.id, b.id]);
  });
});

describe('coverageLabel', () => {
  it('shows covered/total, or a dash when unavailable', () => {
    expect(coverageLabel({ covered: 3, total: 5 })).toBe('3/5');
    expect(coverageLabel(null)).toBe('—');
  });
});

describe('ModelEditorStore coverage overlay', () => {
  let store: ModelEditorStore;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [ModelEditorStore] });
    store = TestBed.inject(ModelEditorStore);
  });

  it('highlights nothing while the overlay is off', () => {
    const a = store.addNode('initial');
    store.setTransitionCoverage({ covered: 0, total: 1, uncoveredTransitionIds: ['e'] });
    expect(store.isUncoveredState(a.id)).toBe(false);
    expect(store.isUncoveredTransition('e')).toBe(false);
  });

  it('follows the editor tests live, without a save', () => {
    const a = store.addNode('initial');
    store.coverageView.set('states');
    expect(store.isUncoveredState(a.id)).toBe(true);

    store.addTest(a.id);

    expect(store.isUncoveredState(a.id)).toBe(false);
    expect(store.stateCoverage().covered).toBe(1);
  });

  it('takes transitions from the saved model and marks them stale after an edit', () => {
    store.coverageView.set('transitions');
    store.setTransitionCoverage({ covered: 1, total: 2, uncoveredTransitionIds: ['e2'] });
    store.clearDirty();
    expect(store.isUncoveredTransition('e2')).toBe(true);
    expect(store.isUncoveredTransition('e1')).toBe(false);
    expect(store.transitionCoverageStale()).toBe(false);

    store.addNode('regular');

    expect(store.transitionCoverageStale()).toBe(true);
  });

  it('shows only the chosen dimension', () => {
    const a = store.addNode('initial');
    store.setTransitionCoverage({ covered: 0, total: 1, uncoveredTransitionIds: ['e'] });
    store.coverageView.set('transitions');
    expect(store.isUncoveredState(a.id)).toBe(false);
    expect(store.isUncoveredTransition('e')).toBe(true);
    store.coverageView.set('states');
    expect(store.isUncoveredState(a.id)).toBe(true);
    expect(store.isUncoveredTransition('e')).toBe(false);
  });

  it('has no transition coverage for a model that was never saved', () => {
    store.coverageView.set('both');
    expect(store.transitionCoverage()).toBeNull();
    expect(store.transitionCoverageStale()).toBe(false);
  });

  it('forgets transition coverage when another model is opened', () => {
    store.setTransitionCoverage({ covered: 1, total: 1, uncoveredTransitionIds: [] });
    store.reset();
    expect(store.transitionCoverage()).toBeNull();
  });
});
