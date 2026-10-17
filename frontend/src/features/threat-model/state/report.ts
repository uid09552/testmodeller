/**
 * The assessment report (FR-082): the same sections, in the same order, as
 * HTML (print) and as Markdown. Pure functions over a threat model snapshot,
 * live or from a baseline.
 */
import {
  Evidence, RiskLevel, RiskThreshold, Threat, ThreatModelSnapshot,
} from '../../../core/api/threat-model-api';
import {
  aiMark, CONTROL_FLAG_TEXT, FLAG_TEXT, label, levelText, LIKELIHOOD, IMPACT, targetName,
} from './threat-model-view';

/** Section headings, in order. The HTML report uses the same list. */
export const REPORT_SECTIONS = [
  'Scope and assumptions',
  'Architecture',
  'Assets',
  'Risk matrices',
  'Risk register',
  'Controls and evidence',
  'Open flags and warnings',
  'Sign-off',
] as const;

export interface ReportInput {
  snapshot: ThreatModelSnapshot;
  /** Set when the report is of a baseline. */
  baseline: { name: string; createdAt: string; note: string | null } | null;
  generatedOn: string;
}

export type MatrixBasis = 'inherent' | 'current' | 'residual';
export const MATRIX_BASES: MatrixBasis[] = ['inherent', 'current', 'residual'];

/** Open and accepted threats: the ones a risk register is about. */
export function liveThreats(threats: Threat[]): Threat[] {
  return threats.filter(t => t.effectiveStatus === 'open' || t.effectiveStatus === 'accepted');
}

/** Candidates and dismissed threats are not part of the risk register. */
export const inRegister = (t: Threat): boolean =>
  t.effectiveStatus !== 'candidate' && t.effectiveStatus !== 'dismissed';

/** Cell counts of one rating basis; unrated threats are in no cell. */
export function matrixCells(threats: Threat[], basis: MatrixBasis) {
  const m = new Map<string, number>();
  for (const t of liveThreats(threats)) {
    const r = t[basis];
    if (r) m.set(`${r.likelihood}:${r.impact}`, (m.get(`${r.likelihood}:${r.impact}`) ?? 0) + 1);
  }
  return [...m].map(([k, count]) => {
    const [likelihood, impact] = k.split(':').map(Number);
    return { likelihood, impact, count };
  });
}

/** Threats the report flags, with the flag texts. */
export function flaggedThreats(threats: Threat[]): { threat: Threat; texts: string[] }[] {
  return threats
    .filter(t => t.flags.length)
    .map(t => ({ threat: t, texts: t.flags.map(f => FLAG_TEXT[f]) }));
}

export function evidenceOf(snapshot: ThreatModelSnapshot, ownerId: string): Evidence[] {
  return (snapshot.evidence ?? []).filter(e => e.ownerId === ownerId);
}

/** All warnings of architecture entities, as "Entity: message". */
export function warningLines(s: ThreatModelSnapshot): string[] {
  const flowName = (id: string) => targetName({ kind: 'flow', id }, s.components, s.flows);
  return [
    ...s.components.flatMap(c => c.warnings.map(w => `${c.name}: ${w.message}`)),
    ...s.flows.flatMap(f => f.warnings.map(w => `${flowName(f.id)}: ${w.message}`)),
    ...s.assets.flatMap(a => a.warnings.map(w => `${a.name}: ${w.message}`)),
  ];
}

const cell = (v: string | number | null | undefined) => String(v ?? '—').replace(/\|/g, '\\|').replace(/\r?\n/g, ' ');
const row = (...c: (string | number | null | undefined)[]) => `| ${c.map(cell).join(' | ')} |`;
const head = (...c: string[]) => `${row(...c)}\n${row(...c.map(() => '---'))}`;

function matrixMarkdown(threats: Threat[], basis: MatrixBasis, thresholds: RiskThreshold[]): string {
  const counts = new Map(matrixCells(threats, basis).map(c => [`${c.likelihood}:${c.impact}`, c.count]));
  const level = (s: number): RiskLevel =>
    thresholds.find(t => s >= t.min && s <= t.max)?.level ?? 'critical';
  const lines = [head('Likelihood \\ Impact', ...IMPACT.map((n, i) => `${i + 1} ${n}`))];
  for (const l of [5, 4, 3, 2, 1]) {
    lines.push(row(`${l} ${LIKELIHOOD[l - 1]}`, ...[1, 2, 3, 4, 5].map(i => {
      const n = counts.get(`${l}:${i}`) ?? 0;
      return `${l * i} ${label(level(l * i))}${n ? ` (${n})` : ''}`;
    })));
  }
  return lines.join('\n');
}

/** The report as Markdown. */
export function reportMarkdown(r: ReportInput): string {
  const s = r.snapshot;
  const m = s.threatModel;
  const out: string[] = [];
  out.push(`# Threat and risk assessment: ${m.name}`);
  out.push(r.baseline
    ? `Baseline **${r.baseline.name}**, frozen ${r.baseline.createdAt.slice(0, 10)}. Report generated ${r.generatedOn}.`
    : `Live model. Report generated ${r.generatedOn}.`);
  if (r.baseline?.note) out.push(`> ${r.baseline.note}`);

  out.push(`## ${REPORT_SECTIONS[0]}`);
  for (const [title, text] of [['Scope', m.scope], ['Objectives', m.objectives],
    ['Assumptions', m.assumptions], ['Exclusions', m.exclusions]] as const) {
    out.push(`**${title}:** ${text?.trim() || '—'}`);
  }

  const zoneName = (id: string | null | undefined) => s.zones.find(z => z.id === id)?.name ?? 'no zone';
  out.push(`## ${REPORT_SECTIONS[1]}`);
  out.push(`### Zones\n${head('Zone', 'Kind', 'Trust level')}\n${s.zones.map(z => row(z.name, label(z.kind), z.trustLevel)).join('\n')}`);
  out.push(`### Components\n${head('Component', 'Type', 'Zone', 'Owner', 'Internet-facing')}\n${s.components.map(c => row(c.name, label(c.type), zoneName(c.zoneId), c.owner, c.internetFacing ? 'yes' : 'no')).join('\n')}`);
  out.push(`### Data flows\n${head('Flow', 'Relationship', 'Authentication', 'Encrypted', 'Crosses trust boundary')}\n${s.flows.map(f => row(targetName({ kind: 'flow', id: f.id }, s.components, s.flows), label(f.relationship), f.authentication, label(f.encrypted), f.crossing.crosses ? `yes (${f.crossing.fromTrust ?? '–'} → ${f.crossing.toTrust ?? '–'})` : 'no')).join('\n')}`);

  out.push(`## ${REPORT_SECTIONS[2]}`);
  out.push(`${head('Asset', 'Category', 'C / I / A', 'Classification', 'Personal data')}\n${s.assets.map(a => row(a.name, label(a.category), [a.confidentiality, a.integrity, a.availability].map(x => label(x) || '–').join(' / '), label(a.classification), a.personalData ? 'yes' : 'no')).join('\n')}`);

  out.push(`## ${REPORT_SECTIONS[3]}`);
  for (const b of MATRIX_BASES) {
    out.push(`### ${label(b)}\n${matrixMarkdown(s.threats, b, m.thresholds)}`);
  }

  out.push(`## ${REPORT_SECTIONS[4]}`);
  out.push(`${head('Threat', 'Status', 'Inherent', 'Current', 'Residual', 'Treatment', 'Owner', 'Due', 'Acceptance')}\n${s.threats.filter(inRegister).map(t => row(
    `${t.displayId} ${t.title}${aiMark(t)}`, label(t.effectiveStatus), levelText(t.inherentLevel), levelText(t.currentLevel),
    levelText(t.residualLevel), label(t.treatment), t.riskOwner, t.dueDate,
    t.acceptedBy ? `${t.acceptedBy}, ${t.acceptedOn ?? '?'} until ${t.acceptanceExpires ?? 'no expiry'}${t.acceptanceRationale ? `: ${t.acceptanceRationale}` : ''}` : '—',
  )).join('\n')}`);

  out.push(`## ${REPORT_SECTIONS[5]}`);
  for (const c of s.controls) {
    out.push(`### ${c.displayId} ${c.name}${aiMark(c)}`);
    out.push(`${label(c.type)}, ${label(c.status)}${c.verifiedAt ? `, verified ${c.verifiedAt.slice(0, 10)}` : ''}${c.reviewBy ? `, review by ${c.reviewBy}` : ''}.`);
    const ev = evidenceOf(s, c.id);
    out.push(ev.length
      ? ev.map(e => `- ${label(e.kind)}: ${e.title} (${e.evidenceDate}${e.reviewer ? `, ${e.reviewer}` : ''})${e.link ? ` <${e.link}>` : ''}${e.note ? ` — ${e.note}` : ''}`).join('\n')
      : '- No evidence.');
  }
  if (!s.controls.length) out.push('No controls.');

  out.push(`## ${REPORT_SECTIONS[6]}`);
  const flags = [
    ...flaggedThreats(s.threats).map(f => `- ${f.threat.displayId} ${f.threat.title}: ${f.texts.join('; ')}`),
    ...s.controls.filter(c => c.flags.length).map(c => `- ${c.displayId} ${c.name}: ${c.flags.map(f => CONTROL_FLAG_TEXT[f]).join('; ')}`),
    ...warningLines(s).map(w => `- ${w}`),
  ];
  out.push(flags.length ? flags.join('\n') : 'None.');

  out.push(`## ${REPORT_SECTIONS[7]}`);
  out.push(`${head('Name', 'Role', 'Date', 'Signature')}\n${row('', '', '', '')}`);
  return out.join('\n\n') + '\n';
}
