/**
 * Reads a chat message: which kind of proposal it asks for, and which
 * threats, controls or tests it names. Pure, so it is easy to check.
 */
import { Control, SecurityTest, Threat, TmProposalKind } from '../../../core/api/threat-model-api';

export const KIND_LABEL: Record<TmProposalKind, string> = {
  'tm-architecture': 'Architecture',
  'tm-threats': 'Threats',
  'tm-ratings': 'Ratings',
  'tm-controls': 'Controls',
  'tm-security-tests': 'Security tests',
  'tm-review': 'Review',
};
export const KINDS = Object.keys(KIND_LABEL) as TmProposalKind[];

/** First matching rule wins; threats are the default. */
const RULES: [RegExp, TmProposalKind][] = [
  [/\b(missing|gaps?|review|inconsisten\w*|audit|incomplete)\b/i, 'tm-review'],
  [/\b(rate|rating|ratings|likelihood|impact|score|severity)\b/i, 'tm-ratings'],
  [/\btests?\b/i, 'tm-security-tests'],
  [/\b(controls?|mitigat\w*|countermeasures?|safeguards?)\b/i, 'tm-controls'],
  [/\b(architecture|components?|zones?|data ?flows?|flows?|diagram|describe|database|gateway|services?|we (have|use|run))\b/i, 'tm-architecture'],
];

/** The kind a message asks for. */
export function inferKind(text: string): TmProposalKind {
  return RULES.find(([re]) => re.test(text))?.[1] ?? 'tm-threats';
}

/** Ids of the threats (THR-n), controls (CTL-n) and tests (SEC-n) a message names. */
export function idsIn(
  text: string, threats: Threat[], controls: Control[], tests: SecurityTest[],
): string[] {
  const out: string[] = [];
  for (const m of text.matchAll(/\b(THR|CTL|SEC)-0*(\d+)\b/gi)) {
    const display = `${m[1].toUpperCase()}-${Number(m[2])}`;
    const hit = [...threats, ...controls, ...tests].find(x => x.displayId === display);
    if (hit && !out.includes(hit.id)) out.push(hit.id);
  }
  return out;
}
