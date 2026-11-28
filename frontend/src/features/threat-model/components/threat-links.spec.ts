import { By } from '@angular/platform-browser';
import { ConvertTestComponent } from './convert-test/convert-test';
import { resultBadge } from './covering-tests/covering-tests';
import { SecurityTestsTabComponent } from './security-tests-tab/security-tests-tab';
import { ThreatsTabComponent } from './threats-tab/threats-tab';
import { test as secTest, threat } from '../testing/fixtures';
import { byText, mount } from '../testing/setup';

/** ADR 0016: threats covered by feature test cases, and converting security tests. */
describe('threat links', () => {
  afterEach(() => vi.restoreAllMocks());

  it('lists the feature test cases that cover a threat, with their latest result', async () => {
    const h = await mount(ThreatsTabComponent, api => {
      api.data.threats = [threat('t1', 1, { status: 'open', testCaseIds: ['tc1', 'tc2'] })];
    });
    h.org.getTestCase.mockImplementation(async (id: string) => ({
      id, version: 1, featureId: 'f1', name: `Case ${id}`, steps: [], assignments: [],
      ...(id === 'tc1' ? { lastResult: { status: 'failed', executedAt: '2026-11-01T00:00:00Z' } } : {}),
    }));
    byText(h.root, 'tbody .link-btn', 'THR-1').click();
    await h.settle();
    await h.settle();
    const section = h.root.querySelector('tm-covering-tests')!;
    expect(section.textContent).toContain('Covered by test cases');
    expect(section.textContent).toContain('Case tc1');
    expect(section.textContent).toContain('✗ Failed');
    expect(section.textContent).toContain('Case tc2');
    expect(section.textContent).toContain('– No result');
    expect(section.querySelector('a')!.getAttribute('href')).toBe('/test-cases?open=tc1');
  });

  it('says when no test case covers a threat', async () => {
    const h = await mount(ThreatsTabComponent, api => { api.data.threats = [threat('t1', 1)]; });
    byText(h.root, 'tbody .link-btn', 'THR-1').click();
    await h.settle();
    expect(h.root.querySelector('tm-covering-tests')!.textContent).toContain('No feature test case covers this threat yet');
    expect(h.org.getTestCase).not.toHaveBeenCalled();
  });

  it('reads results as text, not colour alone', () => {
    const base = { id: 'x', version: 1, featureId: 'f', name: 'n', steps: [], assignments: [] };
    expect(resultBadge({ ...base, lastResult: { status: 'passed' } as never }).text).toBe('✓ Passed');
    expect(resultBadge({ ...base, lastResult: { status: 'error' } as never }).text).toBe('✗ Failed');
    expect(resultBadge(base).text).toBe('– No result');
  });

  async function openConvert(answer = 'convert') {
    const h = await mount(SecurityTestsTabComponent, api => {
      api.data.threats = [threat('t1', 1)];
      api.data['security-tests'] = [secTest('s2', 2, { title: 'Lockout', status: 'ready', threatIds: ['t1'] })];
    });
    h.messages.answer = answer;
    byText(h.root, 'tbody .link-btn', 'SEC-2').click();
    await h.settle();
    const convert = h.fixture.debugElement.query(By.directive(ConvertTestComponent))
      .componentInstance as ConvertTestComponent;
    return { h, convert };
  }

  it('offers the project models and converts into the chosen one after confirming', async () => {
    const { h, convert } = await openConvert();
    expect(convert.targets().map(t => t.label)).toEqual(['Payments › Pay › Checkout']);
    convert.choose(new CustomEvent('valueChange', { detail: 'm1' }));
    await convert.convert();
    await h.settle();
    expect(h.messages.information).toHaveBeenCalledTimes(1);
    expect(h.api.writes('convertTest')[0].args).toEqual(['s2', 'f1', 'm1']);
    expect(h.toasts.show).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
    // The editor closes and the list reloads: the test is obsolete now.
    expect(h.root.querySelector('tm-convert-test')).toBeNull();
    expect(h.store.tests().find(t => t.id === 's2')!.status).toBe('obsolete');
  });

  it('does nothing when the confirmation is cancelled', async () => {
    const { h, convert } = await openConvert('cancel');
    convert.choose(new CustomEvent('valueChange', { detail: 'm1' }));
    await convert.convert();
    expect(h.api.writes('convertTest')).toEqual([]);
  });

  it('ignores a model that is not in the project', async () => {
    const { convert } = await openConvert();
    convert.choose(new CustomEvent('valueChange', { detail: 'elsewhere' }));
    expect(convert.modelId()).toBeNull();
  });

  it('links a converted test to its feature test case', async () => {
    const h = await mount(SecurityTestsTabComponent, api => {
      api.data['security-tests'] = [secTest('s2', 2, { status: 'obsolete', convertedToTestCaseId: 'tc9' })];
    });
    byText(h.root, 'tbody .link-btn', 'SEC-2').click();
    await h.settle();
    const section = h.root.querySelector('tm-convert-test')!;
    expect(section.textContent).toContain('Converted');
    expect(section.querySelector('a')!.getAttribute('href')).toBe('/test-cases?open=tc9');
  });
});
