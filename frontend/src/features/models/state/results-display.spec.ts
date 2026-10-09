import { TestCase } from '../../../core/api/api.types';
import { testFromApi, testToInput } from './model-mapping';
import { isFailing, ModelEditorStore, resultLabel, stateCoverage } from './model-editor.store';

const RESULT = {
  id: 'r1', testCaseId: 't1', runId: 'junit:x', status: 'failed' as const,
  message: 'expected home', executedAt: '2026-10-01T09:00:00Z', source: 'junit',
};

describe('latest result in the editor', () => {
  it('labels every status with a symbol and a word', () => {
    expect(resultLabel('passed')).toBe('✓ Passed');
    expect(resultLabel('failed')).toBe('✗ Failed');
    expect(resultLabel('error')).toBe('⚠ Error');
    expect(resultLabel('skipped')).toBe('– Skipped');
  });

  it('counts failed and error as failing, not skipped', () => {
    expect(isFailing({ status: 'failed', executedAt: '' })).toBe(true);
    expect(isFailing({ status: 'error', executedAt: '' })).toBe(true);
    expect(isFailing({ status: 'skipped', executedAt: '' })).toBe(false);
    expect(isFailing(undefined)).toBe(false);
  });

  it('is read from the API and never sent back', () => {
    const tc: TestCase = {
      id: 't1', version: 1, featureId: 'f', name: 'Login', steps: [], tags: [],
      assignments: [], lastResult: RESULT,
    };
    const t = testFromApi(tc, 1);
    expect(t.lastResult).toEqual({
      status: 'failed', executedAt: '2026-10-01T09:00:00Z', message: 'expected home',
    });
    expect('lastResult' in testToInput(t, 'm', 's')).toBe(false);
  });

  it('counts a covered state as passing only when all its tests last passed', () => {
    const store = new ModelEditorStore();
    const a = store.addNode('initial');
    const b = store.addNode('regular');
    store.addNode('final');
    const ta = store.addTest(a.id)!;
    const tb1 = store.addTest(b.id)!;
    const tb2 = store.addTest(b.id)!;
    expect(store.hasResults()).toBe(false);

    store.updateTest(a.id, ta.id, { lastResult: { status: 'passed', executedAt: '' } });
    store.updateTest(b.id, tb1.id, { lastResult: { status: 'passed', executedAt: '' } });
    store.updateTest(b.id, tb2.id, { lastResult: { status: 'failed', executedAt: '' } });

    const c = stateCoverage(store.nodes());
    expect([c.covered, c.total, c.passing]).toEqual([2, 3, 1]);
    expect(store.hasResults()).toBe(true);
    expect(store.failingCountOn(store.nodeById(b.id)!)).toBe(1);
  });
});
