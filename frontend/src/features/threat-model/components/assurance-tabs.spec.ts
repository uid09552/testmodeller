import { BaselinesTabComponent } from './baselines-tab/baselines-tab';
import { ControlsTabComponent } from './controls-tab/controls-tab';
import { OverviewTabComponent } from './overview-tab/overview-tab';
import { ReportTabComponent } from './report-tab/report-tab';
import { RisksTabComponent } from './risks-tab/risks-tab';
import { ThreatsTabComponent } from './threats-tab/threats-tab';
import { component, control, flow, model, summary, threat } from '../testing/fixtures';
import { byText, mount, setValue } from '../testing/setup';

describe('assurance', () => {
  afterEach(() => vi.restoreAllMocks());

  describe('Risks: residual rating and acceptance', () => {
    const t1 = threat('t1', 1, {
      title: 'Spoofed session', targets: [{ kind: 'component', id: 'a' }],
      inherent: { likelihood: 4, impact: 5 }, inherentScore: 20, inherentLevel: 'critical',
      residual: { likelihood: 2, impact: 3 }, residualScore: 6, residualLevel: 'moderate',
      flags: ['unsupported-residual'], residualFacts: ['CTL-1 is implemented'],
    });

    it('shows the residual score and switches the matrix to residual', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = [t1]; });
      expect(h.root.querySelector('tm-entity-table tbody')!.textContent).toContain('6 ▃ Moderate');
      byText(h.root, 'button.chip', 'Residual').click();
      await h.settle();
      const cell = h.root.querySelector('[aria-label^="Likelihood 2 Unlikely, impact 3 Moderate"]')!;
      expect(cell.getAttribute('aria-label')).toContain('1 threat');
      expect(h.root.querySelector('[aria-label^="Likelihood 4 Likely, impact 5 Severe"]')!.getAttribute('aria-label')).toContain('0 threats');
    });

    it('saves the residual rating together with the other fields', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = [t1]; });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      const sent = h.api.writes('updateItem')[0].args[3] as Record<string, unknown>;
      expect(sent['residual']).toEqual({ likelihood: 2, impact: 3 });
      expect(sent['inherent']).toEqual({ likelihood: 4, impact: 5 });
    });

    it('asks for an expiry when a risk is accepted', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = [t1]; });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      expect(h.root.querySelector('[name="acceptanceExpires"]')).toBeNull();
      setValue(h.root.querySelector<HTMLSelectElement>('[name="treatment"]')!, 'accept');
      await h.settle();
      expect(h.root.querySelector('[name="acceptanceExpires"]')).not.toBeNull();
    });

    it('shows why a residual flag is raised', async () => {
      const h = await mount(RisksTabComponent, api => { api.data.threats = [t1]; });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      expect(h.root.querySelector('tm-editor-drawer')!.textContent).toContain('Residual risk lower than current without a verified control');
      expect(h.root.querySelector('tm-editor-drawer')!.textContent).toContain('CTL-1 is implemented');
    });
  });

  describe('Controls: evidence', () => {
    it('offers verified only with evidence', async () => {
      const h = await mount(ControlsTabComponent, api => {
        api.data.controls = [control('c1', 1, { name: 'MFA', status: 'implemented' })];
      });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      const verified = h.root.querySelector<HTMLOptionElement>('select[name="status"] option[value="verified"]')!;
      expect(verified.disabled).toBe(true);
      expect(h.root.querySelector('tm-editor-drawer')!.textContent).toContain('once it has evidence');
    });

    it('adds and deletes evidence', async () => {
      const h = await mount(ControlsTabComponent, api => {
        api.data.controls = [control('c1', 1, { name: 'MFA', status: 'implemented' })];
      });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      expect(h.root.querySelector('tm-editor-drawer')!.textContent).toContain('No evidence yet');
      setValue(h.root.querySelector<HTMLInputElement>('[name="evTitle"]')!, 'Pen test');
      setValue(h.root.querySelector<HTMLInputElement>('[name="evLink"]')!, 'https://x/report');
      await h.settle();
      byText(h.root, 'button', 'Add evidence').click();
      await h.settle();
      expect(h.api.writes('addControlEvidence')[0].args[1]).toEqual(expect.objectContaining({
        title: 'Pen test', link: 'https://x/report', note: null,
      }));
      expect(h.root.querySelector('tm-editor-drawer')!.textContent).toContain('Pen test');
      // The control the editor was opened with is stale now; verified must be offered anyway.
      expect(h.root.querySelector<HTMLOptionElement>('select[name="status"] option[value="verified"]')!.disabled).toBe(false);
      byText(h.root, 'tm-editor-drawer button', 'Delete').click();
      await h.settle();
      expect(h.api.writes('deleteEvidence')).toHaveLength(1);
      expect(h.root.querySelector('tm-editor-drawer')!.textContent).toContain('No evidence yet');
    });

    it('shows the evidence count and flags in the table', async () => {
      const h = await mount(ControlsTabComponent, api => {
        api.data.controls = [control('c1', 1, { status: 'verified', evidenceCount: 0, flags: ['verified-without-evidence'] })];
      });
      expect(h.root.querySelector('tbody')!.textContent).toContain('Verified without evidence');
    });
  });

  describe('Baselines', () => {
    it('creates, compares and deletes a baseline', async () => {
      const h = await mount(BaselinesTabComponent, api => {
        api.data.threats = [threat('t1', 1)];
        api.comparison = {
          added: [{ id: 'n', displayId: 'THR-9', title: 'New one', changes: [] }], removed: [],
          changed: [{ id: 't1', displayId: 'THR-1', title: 'Spoofed', changes: [{ field: 'currentLevel', before: 'high', after: 'low' }] }],
          controlsChanged: [{ id: 'c1', displayId: 'CTL-1', before: 'planned', after: 'verified' }],
          levelsBaseline: { high: 1 }, levelsLive: { low: 1 },
        };
      });
      setValue(h.root.querySelector<HTMLInputElement>('[name="baselineName"]')!, 'Release 2.3 review');
      h.root.querySelector('form')!.dispatchEvent(new Event('submit'));
      await h.settle();
      expect(h.api.writes('createBaseline')[0].args[0]).toEqual({ name: 'Release 2.3 review', note: null });
      expect(h.root.querySelector('table')!.textContent).toContain('Release 2.3 review');

      byText(h.root, 'button', 'Compare').click();
      await h.settle();
      const cmp = h.root.querySelector('[aria-label="Comparison"]')!.textContent!;
      expect(cmp).toContain('THR-9 New one');
      expect(cmp).toContain('current level');
      expect(cmp).toContain('high → low');
      expect(cmp).toContain('CTL-1: Planned → Verified');

      vi.spyOn(window, 'confirm').mockReturnValue(true);
      byText(h.root, 'table button', 'Delete').click();
      await h.settle();
      expect(h.api.writes('deleteBaseline')).toHaveLength(1);
      expect(h.root.textContent).toContain('No baselines yet');
    });

    it('opens a baseline as a read-only report', async () => {
      const h = await mount(BaselinesTabComponent, api => {
        api.baselineList = [{
          id: 'b1', name: 'R1', note: null, createdAt: '2026-09-30T00:00:00Z', createdBy: 'x',
          threatCount: 0, controlCount: 0, schemaVersion: 1,
        }];
      });
      byText(h.root, 'button', 'Open').click();
      expect(h.nav.go).toHaveBeenCalledWith('report', { baseline: 'b1' });
    });
  });

  describe('Report', () => {
    it('shows the live model with a sign-off block and no baseline banner', async () => {
      const h = await mount(ReportTabComponent, api => {
        api.data.components = [component('a', 'Web')];
        api.data.threats = [threat('t1', 1, { title: 'Spoofed session' })];
      });
      const text = h.root.textContent!;
      for (const heading of ['Scope and assumptions', 'Architecture', 'Risk register', 'Sign-off']) {
        expect(text).toContain(heading);
      }
      expect(h.root.querySelector('.tra-report__banner')).toBeNull();
      expect(text).toContain('THR-1 Spoofed session');
    });

    it('names the baseline and offers no editing', async () => {
      const h = await mount(ReportTabComponent, api => {
        api.baselineDetail = {
          id: 'b1', name: 'Release 2.3 review', note: 'signed off', createdAt: '2026-09-30T10:00:00Z',
          createdBy: 'x', threatCount: 1, controlCount: 0, schemaVersion: 1,
          snapshot: {
            threatModel: model({ name: 'Shop' }), zones: [], components: [], assets: [], flows: [],
            threats: [threat('t1', 1, { title: 'Old threat' })], controls: [], securityTests: [], evidence: [],
          },
        };
      }, { baselineId: 'b1' });
      expect(h.root.querySelector('.tra-report__banner')!.textContent).toContain('Release 2.3 review');
      expect(h.root.textContent).toContain('Old threat');
      expect(h.root.querySelector('input, textarea, select')).toBeNull();
    });

    it('exports Markdown', async () => {
      const h = await mount(ReportTabComponent);
      const click = vi.fn();
      vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:x');
      vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined);
      vi.spyOn(document, 'createElement').mockImplementation(((tag: string) => {
        const el = Document.prototype.createElement.call(document, tag);
        if (tag === 'a') el.click = click;
        return el;
      }) as typeof document.createElement);
      byText(h.root, 'button', 'Export Markdown').click();
      expect(click).toHaveBeenCalled();
    });
  });

  describe('Overview findings', () => {
    it('lists completeness findings and links to the entity', async () => {
      const h = await mount(OverviewTabComponent, api => {
        api.summaryValue = summary({
          components: 1,
          completeness: [{ code: 'COMPONENT_NO_OWNER', message: 'Web has no owner', entityKind: 'component', entityId: 'a' }],
          expiringAcceptances: [{ id: 't1', label: 'THR-1 Spoofed' }], expiredAcceptances: 2, verificationOverdue: 1,
        });
      });
      const text = h.root.textContent!;
      expect(text).toContain('1 open finding(s)');
      expect(text).toContain('THR-1 Spoofed');
      byText(h.root, '.link-btn', 'Web has no owner').click();
      expect(h.nav.go).toHaveBeenCalledWith('components', { select: 'a' });
    });

    it('says when there are no findings', async () => {
      const h = await mount(OverviewTabComponent, api => { api.summaryValue = summary({ components: 1 }); });
      expect(h.root.textContent).toContain('No open findings');
    });
  });

  describe('Detail fields', () => {
    it('saves threat source and confidence and shows affected boundaries', async () => {
      const t = threat('t1', 1, {
        targets: [{ kind: 'component', id: 'a' }],
        affectedBoundaries: [{ crosses: true, direction: 'into-less-trusted', fromTrust: 3, toTrust: 0 }],
      });
      const h = await mount(ThreatsTabComponent, api => {
        api.data.components = [component('a', 'API')];
        api.data.flows = [flow('f1', 'a', 'a')];
        api.data.threats = [t];
      });
      h.root.querySelector<HTMLButtonElement>('tbody .link-btn')!.click();
      await h.settle();
      expect(h.root.querySelector('[aria-label="Affected trust boundaries"]')!.textContent).toContain('trust 3 → 0');
      setValue(h.root.querySelector<HTMLInputElement>('[name="source"]')!, 'malicious tenant admin');
      byText(h.root, 'button', 'Save').click();
      await h.settle();
      expect(h.api.writes('updateItem')[0].args[3]).toEqual(expect.objectContaining({
        threatSource: 'malicious tenant admin',
      }));
    });
  });
});
