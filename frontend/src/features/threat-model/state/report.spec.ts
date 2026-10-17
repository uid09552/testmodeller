import { ThreatModelSnapshot } from '../../../core/api/threat-model-api';
import {
  asset, component, control, flow, model, threat, zone,
} from '../testing/fixtures';
import {
  evidenceOf, flaggedThreats, matrixCells, REPORT_SECTIONS, reportMarkdown, warningLines,
} from './report';

function snapshot(): ThreatModelSnapshot {
  return {
    threatModel: model({ scope: 'The shop', assumptions: 'TLS everywhere' }),
    zones: [zone('z0', 'Internet', 0), zone('z3', 'Internal', 3)],
    components: [component('a', 'Web', { zoneId: 'z0' }), component('b', 'API', { zoneId: 'z3', owner: 'team' })],
    assets: [asset('s1', 'Records', { classification: 'restricted' })],
    flows: [flow('f1', 'a', 'b', {
      crossing: { crosses: true, direction: 'into-more-trusted', fromTrust: 0, toTrust: 3 },
      warnings: [{ code: 'CROSSING_AUTH_UNDOCUMENTED', message: 'authentication not documented' }],
    })],
    threats: [
      threat('t1', 1, {
        title: 'Spoofed session', current: { likelihood: 3, impact: 4 }, currentScore: 12, currentLevel: 'high',
        residual: { likelihood: 2, impact: 2 }, residualScore: 4, residualLevel: 'low',
        inherent: { likelihood: 4, impact: 5 }, inherentScore: 20, inherentLevel: 'critical',
        flags: ['unsupported-residual'],
      }),
      threat('t2', 2, {
        title: 'Accepted one', status: 'accepted', effectiveStatus: 'accepted', treatment: 'accept',
        acceptedBy: 'J. Doe', acceptedOn: '2026-10-01', acceptanceExpires: '2027-01-01',
        acceptanceRationale: 'low exposure', current: { likelihood: 1, impact: 1 }, currentScore: 1, currentLevel: 'low',
      }),
      threat('t3', 3, { title: 'Lapsed', status: 'accepted', effectiveStatus: 'open', flags: ['acceptance-expired'] }),
      threat('t4', 4, { title: 'Dismissed', status: 'dismissed', effectiveStatus: 'dismissed' }),
    ],
    controls: [control('c1', 1, { name: 'MFA', status: 'verified', verifiedAt: '2026-10-02T00:00:00Z', reviewBy: '2027-01-01', evidenceCount: 1 })],
    securityTests: [],
    evidence: [{
      id: 'e1', version: 1, createdAt: '', updatedAt: '', kind: 'test-report', title: 'Pen test', link: 'https://x/r',
      note: null, evidenceDate: '2026-10-01', reviewer: 'R', ownerKind: 'control', ownerId: 'c1',
    }],
  };
}

describe('assessment report', () => {
  it('has the sections in the specified order', () => {
    const md = reportMarkdown({ snapshot: snapshot(), baseline: null, generatedOn: '2026-10-17' });
    const headings = [...md.matchAll(/^## (.+)$/gm)].map(m => m[1]);
    expect(headings).toEqual([...REPORT_SECTIONS]);
    expect(md.startsWith('# Threat and risk assessment: Shop')).toBe(true);
  });

  it('names the baseline and its date', () => {
    const md = reportMarkdown({
      snapshot: snapshot(), generatedOn: '2026-10-17',
      baseline: { name: 'Release 2.3 review', createdAt: '2026-09-30T10:00:00Z', note: 'signed off' },
    });
    expect(md).toContain('Baseline **Release 2.3 review**, frozen 2026-09-30');
    expect(md).toContain('> signed off');
  });

  it('shows ratings, acceptance, evidence, flags and an empty sign-off block', () => {
    const md = reportMarkdown({ snapshot: snapshot(), baseline: null, generatedOn: '2026-10-17' });
    expect(md).toContain('THR-1 Spoofed session');
    expect(md).toContain('J. Doe, 2026-10-01 until 2027-01-01: low exposure');
    expect(md).toContain('- Test report: Pen test (2026-10-01, R) <https://x/r>');
    expect(md).toContain('Residual risk lower than current without a verified control');
    expect(md).toContain('Risk acceptance expired');
    expect(md).toContain('Web → API: authentication not documented');
    expect(md).toContain('| Name | Role | Date | Signature |');
    // A dismissed threat and a candidate are not in the register.
    expect(md).not.toContain('THR-4 Dismissed |');
  });

  it('counts only open and accepted threats with a rating in the matrices', () => {
    const s = snapshot();
    expect(matrixCells(s.threats, 'current')).toEqual([
      { likelihood: 3, impact: 4, count: 1 }, { likelihood: 1, impact: 1, count: 1 },
    ]);
    expect(matrixCells(s.threats, 'residual')).toEqual([{ likelihood: 2, impact: 2, count: 1 }]);
    expect(matrixCells(s.threats, 'inherent')).toEqual([{ likelihood: 4, impact: 5, count: 1 }]);
  });

  it('collects flags, warnings and evidence of the snapshot', () => {
    const s = snapshot();
    expect(flaggedThreats(s.threats).map(f => f.threat.displayId)).toEqual(['THR-1', 'THR-3']);
    expect(warningLines(s)).toEqual(['Web → API: authentication not documented']);
    expect(evidenceOf(s, 'c1')).toHaveLength(1);
    expect(evidenceOf(s, 'other')).toHaveLength(0);
  });
});
