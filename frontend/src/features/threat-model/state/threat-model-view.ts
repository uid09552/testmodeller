/**
 * Pure helpers of the threat modeling workspace: labels, the component type
 * taxonomy, tabs, exports and diagram placement. See
 * docs/specification/09-threat-modeling.md and 06-ui.md.
 */
import {
  ArchComponent, ArchComponentType, ArchLayer, ControlFlag, Flow, RiskLevel, SecurityTest, Threat,
  ThreatFlag, ThreatTarget,
} from '../../../core/api/threat-model-api';

/** Workspace tabs, in order; the slug is the URL segment. */
export const TABS = [
  { slug: 'overview', label: 'Overview' },
  { slug: 'diagram', label: 'Diagram' },
  { slug: 'components', label: 'Components' },
  { slug: 'data-flows', label: 'Data Flows' },
  { slug: 'zones', label: 'Zones' },
  { slug: 'assets', label: 'Assets' },
  { slug: 'threats', label: 'Threats' },
  { slug: 'risks', label: 'Risks' },
  { slug: 'controls', label: 'Controls' },
  { slug: 'security-tests', label: 'Security Tests' },
  { slug: 'baselines', label: 'Baselines' },
  { slug: 'report', label: 'Report' },
] as const;
export type TabSlug = (typeof TABS)[number]['slug'];

export function isTab(s: string | null | undefined): s is TabSlug {
  return TABS.some(t => t.slug === s);
}

/** "information-disclosure" → "Information disclosure". */
export function humanize(value: string | null | undefined): string {
  if (!value) return '';
  const s = value.replace(/-/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}

const SPECIAL: Record<string, string> = {
  api: 'REST/GraphQL API', 'api-gateway': 'API gateway', 'cicd-pipeline': 'CI/CD pipeline',
  'third-party-saas': 'Third-party SaaS', dmz: 'DMZ', cicd: 'CI/CD',
  'identity-security': 'Identity and security', 'not-run': 'Not run',
};

/** Display label of an enum value. */
export function label(value: string | null | undefined): string {
  return value ? SPECIAL[value] ?? humanize(value) : '';
}

/** Component types grouped by layer, in the order the editor offers them. */
export const TYPE_GROUPS: { layer: ArchLayer; types: ArchComponentType[] }[] = [
  { layer: 'client', types: ['web-frontend', 'mobile-app', 'desktop-client', 'browser-extension'] },
  { layer: 'integration', types: ['api-gateway', 'api', 'message-broker', 'webhook', 'external-connector'] },
  { layer: 'logic', types: ['service', 'worker', 'scheduler', 'serverless-function'] },
  { layer: 'data', types: ['relational-database', 'object-storage', 'cache', 'search-index', 'file-store', 'backup'] },
  { layer: 'identity-security', types: ['identity-provider', 'authorization-service', 'secrets-manager', 'key-management', 'audit-service'] },
  { layer: 'platform', types: ['cloud-service', 'container-platform', 'cicd-pipeline', 'third-party-saas', 'library'] },
  { layer: 'external', types: ['human-user', 'external-system'] },
];

export function layerOf(t: ArchComponentType): ArchLayer {
  return TYPE_GROUPS.find(g => g.types.includes(t))?.layer ?? 'logic';
}

/** Marks something that came from an accepted AI proposal; text, never colour alone. */
export function aiMark(x: { origin?: string; source?: string | null }): string {
  return x.origin === 'ai' ? ' ✦ AI' : '';
}

/** Risk level as a symbol and a word, never colour alone. */
export const LEVEL_TEXT: Record<RiskLevel, string> = {
  low: '▁ Low', moderate: '▃ Moderate', high: '▆ High', critical: '█ Critical',
};
export const LEVEL_ORDER: RiskLevel[] = ['low', 'moderate', 'high', 'critical'];

export function levelText(l: RiskLevel | null | undefined): string {
  return l ? LEVEL_TEXT[l] : 'Not assessed';
}

/** Highest level among the given ones, or null. */
export function highestLevel(levels: (RiskLevel | null)[]): RiskLevel | null {
  let best = -1;
  for (const l of levels) if (l) best = Math.max(best, LEVEL_ORDER.indexOf(l));
  return best < 0 ? null : LEVEL_ORDER[best];
}

export const FLAG_TEXT: Record<ThreatFlag, string> = {
  'no-control': 'No control',
  'planned-controls-only': 'Controls planned only',
  'unsupported-reduction': 'Current risk lower than inherent without an implemented control',
  overdue: 'Overdue',
  'failing-test': '✗ Failing test',
  'residual-above-current': 'Residual risk above current risk',
  'unsupported-residual': 'Residual risk lower than current without a verified control',
  'acceptance-expired': 'Risk acceptance expired',
};

/** Flags the Risks tab explains: the residual ones and a lapsed acceptance. */
export function isRiskFlag(f: ThreatFlag): boolean {
  return f === 'residual-above-current' || f === 'unsupported-residual' || f === 'acceptance-expired';
}

export const CONTROL_FLAG_TEXT: Record<ControlFlag, string> = {
  'verification-overdue': 'Verification overdue',
  'verified-without-evidence': 'Verified without evidence',
};

/** Likelihood and impact scale names (09-threat-modeling.md). */
export const LIKELIHOOD = ['Rare', 'Unlikely', 'Possible', 'Likely', 'Almost certain'];
export const IMPACT = ['Negligible', 'Minor', 'Moderate', 'Major', 'Severe'];

/** Display name of a threat target. */
export function targetName(
  t: ThreatTarget, components: ArchComponent[], flows: Flow[],
): string {
  if (t.kind === 'component') return components.find(c => c.id === t.id)?.name ?? '(deleted)';
  const f = flows.find(x => x.id === t.id);
  if (!f) return '(deleted)';
  const n = (id: string) => components.find(c => c.id === id)?.name ?? '?';
  return `${n(f.sourceId)} → ${n(f.targetId)}`;
}

/** Threats that are open (or accepted) and target the component or flow. */
export function openThreatsOn(id: string, threats: Threat[]): Threat[] {
  return threats.filter(t =>
    (t.status === 'open' || t.status === 'accepted') && t.targets.some(x => x.id === id));
}

// ── CSV and Gherkin ────────────────────────────────────────────────────────

function cell(v: string | number | null | undefined): string {
  const s = v == null ? '' : String(v);
  return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

export function csv(rows: (string | number | null | undefined)[][]): string {
  return rows.map(r => r.map(cell).join(',')).join('\r\n') + '\r\n';
}

/** Risk register: one row per assessed threat. */
export function riskRegisterCsv(threats: Threat[]): string {
  const assessed = threats.filter(t => t.inherent || t.current || t.residual);
  return csv([
    ['id', 'title', 'status', 'inherentScore', 'inherentLevel', 'currentScore', 'currentLevel',
      'residualScore', 'residualLevel', 'treatment', 'riskOwner', 'dueDate', 'acceptedBy',
      'acceptanceExpires', 'origin', 'source'],
    ...assessed.map(t => [
      t.displayId, t.title, t.effectiveStatus ?? t.status, t.inherentScore, t.inherentLevel,
      t.currentScore, t.currentLevel, t.residualScore, t.residualLevel, t.treatment, t.riskOwner,
      t.dueDate, t.acceptedBy, t.acceptanceExpires, t.origin, t.source,
    ]),
  ]);
}

/** Security tests: one row per test, steps joined. */
export function securityTestsCsv(tests: SecurityTest[], threats: Threat[]): string {
  const ids = (xs: string[]) => xs.map(id => threats.find(t => t.id === id)?.displayId ?? id).join(' ');
  return csv([
    ['id', 'title', 'type', 'status', 'threats', 'preconditions', 'steps', 'lastResult', 'origin', 'source'],
    ...tests.map(t => [
      t.displayId, t.title, t.type, t.status, ids(t.threatIds), t.preconditions.join('; '),
      t.steps.map((s, i) => `${i + 1}. ${s.action} => ${s.expected}`).join(' | '),
      t.lastResult?.outcome ?? '', t.origin, t.source,
    ]),
  ]);
}

/** Gherkin: one scenario per test, tagged `@sec-<id>` and with its threats. */
export function securityTestsGherkin(tests: SecurityTest[], threats: Threat[], feature: string): string {
  const one = (s: string) => s.replace(/\s*\n\s*/g, ' ').trim();
  const out = [`Feature: ${one(feature)}`, ''];
  for (const t of tests) {
    const tags = [`@sec-${t.id}`, ...t.threatIds.map(id => `@${threats.find(x => x.id === id)?.displayId ?? id}`)];
    out.push(`  ${tags.join(' ')}`);
    out.push(`  Scenario: ${one(t.title)}`);
    t.preconditions.forEach((p, i) => out.push(`    ${i ? 'And' : 'Given'} ${one(p)}`));
    for (const s of t.steps) {
      out.push(`    When ${one(s.action)}`);
      if (s.expected.trim()) out.push(`    Then ${one(s.expected)}`);
    }
    out.push('');
  }
  return out.join('\n');
}

/** Triggers a file download in the browser. */
export function download(name: string, content: string, type: string): void {
  const url = URL.createObjectURL(new Blob([content], { type }));
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  URL.revokeObjectURL(url);
}

// ── Diagram ────────────────────────────────────────────────────────────────

export interface Rect { x: number; y: number; w: number; h: number }

/** The smallest region that contains the point, by id. */
export function regionAt(p: { x: number; y: number }, regions: Map<string, Rect>): string | null {
  let best: string | null = null;
  let area = Infinity;
  for (const [id, r] of regions) {
    if (p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h && r.w * r.h < area) {
      best = id;
      area = r.w * r.h;
    }
  }
  return best;
}

const overlaps = (a: Rect, b: Rect, gap = 16) =>
  a.x < b.x + b.w + gap && b.x < a.x + a.w + gap && a.y < b.y + b.h + gap && b.y < a.y + a.h + gap;

/**
 * A free spot of size w×h: inside `within` when given, else to the right of
 * everything, scanning a grid until nothing overlaps.
 */
export function freeSpot(taken: Rect[], w: number, h: number, within?: Rect): { x: number; y: number } {
  const step = 24;
  const x0 = within ? within.x + 24 : 40;
  const y0 = within ? within.y + 40 : 40;
  const maxX = within ? within.x + within.w - w - 16 : 40 + 12 * (w + 48);
  for (let row = 0; row < 200; row++) {
    for (let x = x0; x <= Math.max(x0, maxX); x += step) {
      const r = { x, y: y0 + row * step, w, h };
      if (!taken.some(t => overlaps(r, t))) return { x: r.x, y: r.y };
    }
  }
  return { x: x0, y: y0 + taken.length * (h + 24) };
}
