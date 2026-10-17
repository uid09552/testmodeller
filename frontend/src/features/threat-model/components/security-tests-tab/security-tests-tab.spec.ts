import { SecurityTestsTabComponent } from './security-tests-tab';
import { test as secTest, threat } from '../../testing/fixtures';
import { byText, mount, setValue } from '../../testing/setup';

describe('SecurityTestsTabComponent', () => {
  afterEach(() => vi.restoreAllMocks());

  const tests = [
    secTest('s1', 1, { title: 'Reject unauthenticated calls', status: 'draft', origin: 'generated', templateId: 'AUTH-FLOW', threatIds: ['t1'] }),
    secTest('s2', 2, { title: 'Deny access to other users', status: 'ready', threatIds: ['t1'],
      lastResult: { id: 'r', securityTestId: 's2', outcome: 'failed', note: 'IDOR', executedAt: '2026-10-17T00:00:00Z' } }),
    secTest('s3', 3, { title: 'Old', stale: 'threat dismissed', origin: 'generated', threatIds: [] }),
  ];

  it('lists tests with their latest result and staleness, never by colour alone', async () => {
    const h = await mount(SecurityTestsTabComponent, api => { api.data['security-tests'] = tests; api.data.threats = [threat('t1', 1)]; });
    const text = h.root.querySelector('tbody')!.textContent!;
    expect(text).toContain('✗ Failed');
    expect(text).toContain('⚠ Stale: threat dismissed');
    expect(text).toContain('THR-1');
  });

  it('generates drafts for the chosen threats and reports skipped ones', async () => {
    const h = await mount(SecurityTestsTabComponent, api => {
      api.data.threats = [threat('t1', 1), threat('t2', 2, { status: 'candidate' })];
      api.generateResult = { created: [tests[0]], skipped: 2 };
    });
    // Candidates are not offered.
    expect(h.root.querySelector('.st__pick')!.textContent).not.toContain('THR-2');
    byText(h.root, 'button', 'Generate tests').click();
    await h.settle();
    expect(h.api.writes('generateTests')[0].args).toEqual([undefined]);
    expect(h.root.textContent).toContain('1 draft test(s) generated, 2 skipped');
    byText<HTMLLabelElement>(h.root, '.st__pick label', 'THR-1').querySelector('input')!.click();
    await h.settle();
    byText(h.root, 'button', 'Generate tests for 1 threat(s)').click();
    await h.settle();
    expect(h.api.writes('generateTests')[1].args).toEqual([['t1']]);
  });

  it('marks drafts ready in bulk', async () => {
    const h = await mount(SecurityTestsTabComponent, api => { api.data['security-tests'] = tests; });
    h.root.querySelector<HTMLInputElement>('tbody tr input[type="checkbox"]')!.click();
    await h.settle();
    byText(h.root, 'button', 'Mark ready').click();
    await h.settle();
    expect(h.api.writes('bulkTestStatus')[0].args).toEqual([['s1'], 'ready']);
  });

  it('records a result of a ready test', async () => {
    const h = await mount(SecurityTestsTabComponent, api => { api.data['security-tests'] = tests; });
    byText(h.root, 'tbody .link-btn', 'SEC-2').click();
    await h.settle();
    byText(h.root, 'button', 'Record result').click();
    await h.settle();
    h.root.querySelector<HTMLInputElement>('input[type="radio"][value="failed"]')!.click();
    setValue(h.root.querySelector<HTMLTextAreaElement>('textarea[name="note"]')!, 'IDOR on /customers/{id}');
    h.root.querySelector<HTMLButtonElement>('.dialog button[type="submit"]')!.click();
    await h.settle();
    expect(h.api.writes('recordResult')[0].args).toEqual(['s2', { outcome: 'failed', note: 'IDOR on /customers/{id}', executedAt: undefined }]);
  });

  it('does not offer results for a draft', async () => {
    const h = await mount(SecurityTestsTabComponent, api => { api.data['security-tests'] = tests; });
    byText(h.root, 'tbody .link-btn', 'SEC-1').click();
    await h.settle();
    expect([...h.root.querySelectorAll('button')].some(b => b.textContent?.includes('Record result'))).toBe(false);
  });

  it('requires a threat and saves steps of a manual test', async () => {
    const h = await mount(SecurityTestsTabComponent, api => { api.data.threats = [threat('t1', 1)]; });
    byText(h.root, 'button', 'Add test').click();
    await h.settle();
    setValue(h.root.querySelector<HTMLInputElement>('input[name="title"]')!, "Access another customer's record");
    setValue(h.root.querySelector<HTMLInputElement>('input[aria-label="Step 1 action"]')!, 'GET /customers/2 as customer 1');
    setValue(h.root.querySelector<HTMLInputElement>('input[aria-label="Step 1 expected result"]')!, '403');
    byText<HTMLLabelElement>(h.root, '.mp label', 'THR-1').querySelector('input')!.click();
    await h.settle();
    byText(h.root, 'button', 'Save').click();
    await h.settle();
    expect(h.api.writes('createItem')[0].args[1]).toEqual(expect.objectContaining({
      title: "Access another customer's record", threatIds: ['t1'],
      steps: [{ action: 'GET /customers/2 as customer 1', expected: '403' }],
    }));
  });

  it('exports exactly the filtered tests as Gherkin', async () => {
    const h = await mount(SecurityTestsTabComponent, api => { api.data['security-tests'] = tests; api.data.threats = [threat('t1', 1)]; });
    setValue(h.root.querySelector<HTMLSelectElement>('select[aria-label="Latest result"]')!, 'failed');
    await h.settle();
    const create = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:x');
    vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined);
    byText(h.root, 'button', 'Export Gherkin').click();
    const text = await (create.mock.calls[0][0] as Blob).text();
    expect(text).toContain('@sec-s2 @THR-1');
    expect(text).not.toContain('@sec-s1');
  });
});
