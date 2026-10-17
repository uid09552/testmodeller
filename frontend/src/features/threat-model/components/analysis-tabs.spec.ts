import { ControlsTabComponent } from './controls-tab/controls-tab';
import { OverviewTabComponent } from './overview-tab/overview-tab';
import { RisksTabComponent } from './risks-tab/risks-tab';
import { ThreatsTabComponent } from './threats-tab/threats-tab';
import { component, control, summary, threat } from '../testing/fixtures';
import { byText, mount, setValue } from '../testing/setup';

const candidate = threat('t1', 1, {
  status: 'candidate', origin: 'rule', title: 'Spoofed caller on Web app → Customer API',
  targets: [{ kind: 'component', id: 'a' }],
  explanation: {
    ruleId: 'SPOOF-BOUNDARY-AUTH', ruleVersion: 1, ruleName: 'Unauthenticated flow into a more trusted zone',
    facts: ['crosses Web app (Internet, trust 0) → Customer API (Internal, trust 3)', 'authentication: unknown'],
    missing: ['Document the authentication method to confirm or dismiss.'],
    recommendations: ['Service authentication'],
  },
});

describe('analysis tabs', () => {
  afterEach(() => vi.restoreAllMocks());

  describe('Threats', () => {
    it('labels candidates and explains why they were suggested', async () => {
      const h = await mount(ThreatsTabComponent, api => {
        api.data.components = [component('a', 'Customer API')];
        api.data.threats = [candidate, threat('t2', 2, { title: 'Open one' })];
      });
      expect(h.root.querySelector('tbody')!.textContent).toContain('◌ Candidate');
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      const why = h.root.querySelector('[aria-label="Why this was suggested"]')!;
      expect(why.textContent).toContain('Unauthenticated flow into a more trusted zone');
      expect(why.textContent).toContain('trust 0');
      expect(why.textContent).toContain('authentication: unknown');
      expect(why.textContent).toContain('Document the authentication method to confirm or dismiss.');
    });

    it('creates a planned control from a recommendation, linked to the threat', async () => {
      const h = await mount(ThreatsTabComponent, api => {
        api.data.components = [component('a', 'Customer API')];
        api.data.threats = [candidate];
      });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      byText(h.root, 'button', 'Add as control').click();
      await h.settle();
      expect(h.api.writes('createItem')[0].args).toEqual(['controls', expect.objectContaining({
        name: 'Service authentication', status: 'planned', threatIds: ['t1'],
      })]);
    });

    it('suggests threats and says what happened', async () => {
      const h = await mount(ThreatsTabComponent, api => { api.suggestResult = { rulesetVersion: 1, created: [candidate], skipped: 0 }; });
      byText(h.root, 'button', 'Suggest threats').click();
      await h.settle();
      expect(h.root.querySelector('[role="status"]')?.textContent).toContain('1 candidate threat(s) suggested');
      h.api.suggestResult = { rulesetVersion: 1, created: [], skipped: 1 };
      byText(h.root, 'button', 'Suggest threats').click();
      await h.settle();
      expect(h.root.querySelector('[role="status"]')?.textContent).toContain('No new candidates');
    });

    it('confirms several candidates at once and asks why when dismissing', async () => {
      const h = await mount(ThreatsTabComponent, api => {
        api.data.threats = [candidate, threat('t2', 2, { status: 'candidate', origin: 'rule' })];
      });
      h.root.querySelector<HTMLInputElement>('thead input[type="checkbox"]')!.click();
      await h.settle();
      byText(h.root, 'button', 'Confirm').click();
      await h.settle();
      expect(h.api.writes('bulkThreatStatus')[0].args).toEqual([['t1', 't2'], 'open', undefined]);

      h.root.querySelector<HTMLInputElement>('thead input[type="checkbox"]')!.click();
      await h.settle();
      vi.spyOn(window, 'prompt').mockReturnValue('');
      byText(h.root, 'button', 'Dismiss').click();
      await h.settle();
      expect(h.api.writes('bulkThreatStatus')).toHaveLength(1);
      vi.spyOn(window, 'prompt').mockReturnValue('behind CDN');
      byText(h.root, 'button', 'Dismiss').click();
      await h.settle();
      expect(h.api.writes('bulkThreatStatus')[1].args).toEqual([['t1', 't2'], 'dismissed', 'behind CDN']);
    });

    it('filters by target, as the diagram badge asks', async () => {
      const h = await mount(ThreatsTabComponent, api => {
        api.data.components = [component('a', 'API'), component('b', 'DB')];
        api.data.threats = [
          threat('t1', 1, { targets: [{ kind: 'component', id: 'a' }] }),
          threat('t2', 2, { targets: [{ kind: 'component', id: 'b' }] }),
        ];
      }, { targetId: 'b' });
      const rows = h.root.querySelectorAll('tbody tr');
      expect(rows).toHaveLength(1);
      expect(rows[0].textContent).toContain('THR-2');
    });

    it('marks threats without controls', async () => {
      const h = await mount(ThreatsTabComponent, api => {
        api.data.threats = [threat('t1', 1, { flags: ['no-control'] }), threat('t2', 2, { flags: ['planned-controls-only'] })];
      });
      expect(h.root.textContent).toContain('No control');
      expect(h.root.textContent).toContain('Controls planned only');
    });
  });

  describe('Risks', () => {
    const rated = [
      threat('t1', 1, { current: { likelihood: 4, impact: 5 }, currentScore: 20, currentLevel: 'critical', inherent: { likelihood: 5, impact: 5 }, inherentScore: 25, inherentLevel: 'critical' }),
      threat('t2', 2, { current: { likelihood: 2, impact: 2 }, currentScore: 4, currentLevel: 'low', dueDate: '2026-01-01', flags: ['overdue'], treatment: 'mitigate' }),
      threat('t3', 3, { status: 'dismissed', current: { likelihood: 4, impact: 5 } }),
    ];

    it('counts open and accepted threats per cell and filters the register by cell', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = rated; });
      const cell = h.root.querySelector<HTMLButtonElement>('button[aria-label^="Likelihood 4 Likely, impact 5 Severe"]')!;
      expect(cell.getAttribute('aria-label')).toBe('Likelihood 4 Likely, impact 5 Severe, Critical, 1 threat');
      cell.click();
      await h.settle();
      const rows = h.root.querySelectorAll('.rt__register tbody tr');
      expect(rows).toHaveLength(1);
      expect(rows[0].textContent).toContain('THR-1');
    });

    it('switches the matrix to the inherent rating', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = rated; });
      byText(h.root, '.chip', 'Inherent').click();
      await h.settle();
      expect(h.root.querySelector('button[aria-label^="Likelihood 5 Almost certain, impact 5"]')!.getAttribute('aria-label')).toContain('1 threat');
    });

    it('marks overdue threats', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = rated; });
      expect(h.root.textContent).toContain('⚠ Overdue');
    });

    it('asks for a rationale and an approver to accept a risk', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = rated; });
      h.root.querySelector<HTMLButtonElement>('.rt__register tbody .link-btn')!.click();
      await h.settle();
      setValue(h.root.querySelector<HTMLSelectElement>('select[name="treatment"]')!, 'accept');
      await h.settle();
      expect(h.root.querySelector('textarea[name="rationale"]')).not.toBeNull();
      expect(h.root.querySelector('input[name="acceptedBy"]')).not.toBeNull();
    });

    it('shows why changed levels were rejected', async () => {
      const h = await mount(RisksTabComponent);
      byText(h.root, 'button', 'Change levels').click();
      await h.settle();
      h.api.failNext = { status: 422, message: 'thresholds: score 9 has no level' };
      byText(h.root, 'button', 'Save levels').click();
      await h.settle();
      expect(h.root.querySelector('.rt__error')?.textContent).toContain('score 9 has no level');
    });

    it('exports the register as CSV', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = rated; });
      const create = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:x');
      vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined);
      byText(h.root, 'button', 'Export CSV').click();
      const blob = create.mock.calls[0][0] as Blob;
      const text = await blob.text();
      expect(text.split('\r\n')[0]).toContain('inherentScore');
      expect(text).toContain('THR-1');
      expect(text).not.toContain('THR-3');
    });
  });

  describe('Controls', () => {
    it('filters controls without tests and shows the verification date', async () => {
      const h = await mount(ControlsTabComponent, api => {
        api.data.controls = [
          control('c1', 1, { securityTestIds: ['s1'], status: 'verified', verifiedAt: '2026-10-10T08:00:00Z' }),
          control('c2', 2),
        ];
      });
      expect(h.root.textContent).toContain('Verified (2026-10-10)');
      setValue(h.root.querySelector<HTMLSelectElement>('select[aria-label="Coverage gap"]')!, 'no-tests');
      await h.settle();
      const rows = h.root.querySelectorAll('tbody tr');
      expect(rows).toHaveLength(1);
      expect(rows[0].textContent).toContain('CTL-2');
    });

    it('links a control to threats', async () => {
      const h = await mount(ControlsTabComponent, api => { api.data.threats = [threat('t1', 1)]; });
      byText(h.root, 'button', 'Add control').click();
      await h.settle();
      setValue(h.root.querySelector<HTMLInputElement>('input[name="name"]')!, 'Object-level authorization');
      byText<HTMLLabelElement>(h.root, 'label', 'THR-1').querySelector('input')!.click();
      await h.settle();
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      expect(h.api.writes('createItem')[0].args[1]).toEqual(expect.objectContaining({
        name: 'Object-level authorization', threatIds: ['t1'], status: 'planned',
      }));
    });
  });

  describe('Overview', () => {
    it('shows the counts and links gaps to their item', async () => {
      const h = await mount(OverviewTabComponent, api => {
        api.summaryValue = summary({
          components: 4, crossings: 1, threatsByStatus: { open: 3 }, openByLevel: { critical: 1 },
          threatsWithoutControl: [{ id: 't9', label: 'THR-9 Unprotected' }], controlsVerifiedRatio: 0.5,
        });
      });
      expect(h.root.textContent).toContain('Open threats');
      expect(h.root.textContent).toContain('50%');
      byText(h.root, 'button', 'THR-9 Unprotected').click();
      expect(h.nav.go).toHaveBeenCalledWith('threats', { select: 't9' });
    });

    it('prompts for the first component of an empty model', async () => {
      const h = await mount(OverviewTabComponent);
      expect(h.root.textContent).toContain('Register the first component');
      expect(h.root.querySelector('[role="alert"]')).toBeNull();
    });

    it('saves the scope fields', async () => {
      const h = await mount(OverviewTabComponent);
      setValue(h.root.querySelector<HTMLTextAreaElement>('textarea[name="assumptions"]')!, 'TLS everywhere');
      await h.settle();
      byText(h.root, 'button', 'Save scope').click();
      await h.settle();
      expect(h.api.writes('update')[0].args[2]).toEqual(expect.objectContaining({ assumptions: 'TLS everywhere' }));
      expect(h.root.textContent).toContain('Saved.');
    });
  });
});

