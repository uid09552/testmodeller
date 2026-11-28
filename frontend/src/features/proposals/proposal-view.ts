import {
  ElementsPayload, FeatureDescriptionPayload, ModelInput, Proposal, ProposalKind, TestCaseInput,
} from '../../core/api/api.types';

export const KIND_LABEL: Record<ProposalKind, string> = {
  'model': 'Model',
  'states-and-transitions': 'States and transitions',
  'feature-description': 'Feature description',
  'test-cases': 'Test cases',
};

function count(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** A one-line summary of what accepting would add. */
export function summary(p: Proposal): string {
  const items = details(p);
  switch (p.kind) {
    case 'model': {
      const m = p.payload as Partial<ModelInput>;
      return `Model "${m.name ?? 'untitled'}": ${count(m.states?.length ?? 0, 'state', 'states')}, `
        + count(m.transitions?.length ?? 0, 'transition', 'transitions');
    }
    case 'states-and-transitions': {
      const e = p.payload as Partial<ElementsPayload>;
      return `${count(e.states?.length ?? 0, 'state', 'states')}, `
        + count(e.transitions?.length ?? 0, 'transition', 'transitions');
    }
    case 'test-cases':
      return `Test case "${items[0] ?? 'unnamed'}"`;
    case 'feature-description':
      return 'A scenario description';
  }
}

/** What accepting would add, one line each. Unknown shapes give no lines rather than throwing. */
export function details(p: Proposal): string[] {
  const payload = (p.payload ?? {}) as Record<string, unknown>;
  switch (p.kind) {
    case 'model':
    case 'states-and-transitions': {
      const e = payload as Partial<ElementsPayload>;
      const states = (e.states ?? []).map(s => `State "${s.name}" (${s.kind})`);
      const names = new Map((e.states ?? []).map(s => [s.id ?? s.name, s.name]));
      const transitions = (e.transitions ?? []).map(t =>
        `Transition ${names.get(t.from) ?? t.from} → ${names.get(t.to) ?? t.to} on "${t.event}"`
        + (t.guard ? ` [${t.guard}]` : ''));
      return [...states, ...transitions];
    }
    case 'test-cases': {
      // One proposal is one test case (a TestCaseInput).
      const t = payload as Partial<TestCaseInput>;
      return [t.name ?? 'Unnamed test case', ...(t.steps ?? []).map(s => `${s.action} → ${s.expected}`)];
    }
    case 'feature-description':
      return [(payload as Partial<FeatureDescriptionPayload>).scenarioDescription ?? ''].filter(Boolean);
  }
}
