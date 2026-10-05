/**
 * Pure mapping between the API's AI payloads and the editor's canvas model.
 *
 * Kept free of Angular so it can be tested directly, and separate from the
 * chat store so the store is only orchestration.
 */
import {
  ElementsPayload, ModelInput, ProposalKind, StateInput, TestCaseInput,
} from '../../../core/api/api.types';
import {
  StateKind, TestCategory, TestPolarity,
} from './model-editor.store';

/** What the composer offers: an explicit kind, or let the message decide. */
export type ChatIntent = ProposalKind | 'auto';

const KIND_PATTERNS: { kind: ProposalKind; pattern: RegExp }[] = [
  // Checked in order; the first match wins.
  { kind: 'feature-description',   pattern: /\b(scenario|scenarios|description|story|stories|requirement|requirements|acceptance criteria)\b/i },
  { kind: 'model',                 pattern: /\b(whole|complete|entire|new)\s+model\b|\bmodel from scratch\b/i },
  { kind: 'states-and-transitions', pattern: /\b(state|states|transition|transitions|diagram|graph|step|steps|flow)\b/i },
  { kind: 'test-cases',            pattern: /\b(test|tests|test case|test cases|edge case|negative)\b/i },
];

/**
 * Which proposal kind a free-text message is asking for.
 *
 * Test cases are the default: that is what the editor is for, and the user can
 * always override the guess in the composer.
 */
export function inferKind(message: string): ProposalKind {
  for (const { kind, pattern } of KIND_PATTERNS) {
    if (pattern.test(message)) return kind;
  }
  return 'test-cases';
}

/** Human-readable label for a kind, for the composer and the cards. */
export function kindLabel(kind: ProposalKind): string {
  switch (kind) {
    case 'model':                  return 'Whole model';
    case 'states-and-transitions': return 'States & transitions';
    case 'test-cases':             return 'Test cases';
    case 'feature-description':    return 'Scenario description';
  }
}

/** A test case as the canvas stores it, before it gets an id and a sequence. */
export interface TestDraftFromApi {
  name: string;
  category: TestCategory;
  polarity: TestPolarity;
  given: string;
  when: string;
  then: string;
}

const CATEGORY_TAGS: TestCategory[] = ['unit', 'integration', 'feature'];

/**
 * Converts a `test-cases` payload into the editor's Gherkin shape.
 *
 * The API models a test case as ordered action/expected steps plus
 * preconditions; the canvas models it as Given/When/Then. Preconditions become
 * Given, every action becomes a When line and every expectation a Then line,
 * so nothing in the payload is dropped.
 */
export function testCaseToDraft(input: TestCaseInput): TestDraftFromApi {
  const tags = (input.tags ?? []).map(t => t.toLowerCase());
  const given = [input.preconditions, input.description]
    .map(v => v?.trim())
    .filter((v): v is string => !!v)
    .join('\n');
  const steps = input.steps ?? [];
  return {
    name: input.name?.trim() || 'Proposed test case',
    category: CATEGORY_TAGS.find(c => tags.includes(c)) ?? 'feature',
    polarity: tags.some(t => t === 'negative' || t === 'edge') ? 'negative' : 'positive',
    given,
    when: steps.map(s => s.action?.trim()).filter(Boolean).join('\n'),
    then: steps.map(s => s.expected?.trim()).filter(Boolean).join('\n'),
  };
}

/**
 * API state kinds to canvas state kinds.
 *
 * The API has three (`initial`, `normal`, `final`); the canvas adds `decision`,
 * which the contract cannot express, so a proposed state is never a decision.
 */
export function toCanvasKind(kind: StateInput['kind']): StateKind {
  switch (kind) {
    case 'initial': return 'initial';
    case 'final':   return 'final';
    default:        return 'regular';
  }
}

/** One proposed state, resolved onto the canvas. */
export interface ElementDraft {
  /** Key used by the transitions in the same payload. */
  key: string;
  name: string;
  description?: string;
  kind: StateKind;
  position?: { x: number; y: number };
}

export interface TransitionDraft {
  from: string;
  to: string;
  event: string;
  guard?: string;
  action?: string;
}

export interface GraphDraft {
  states: ElementDraft[];
  transitions: TransitionDraft[];
}

/**
 * Normalises a `states-and-transitions` or `model` payload into one shape.
 *
 * Transitions reference states by the id in the payload; proposed states may
 * have no id, so the index is used as a fallback key and transitions that
 * point at an unknown state are dropped rather than creating dangling edges.
 */
export function toGraphDraft(payload: ElementsPayload | ModelInput): GraphDraft {
  const rawStates = payload.states ?? [];
  const states: ElementDraft[] = rawStates.map((s, i) => ({
    key: s.id ?? `#${i}`,
    name: s.name?.trim() || `State ${i + 1}`,
    description: s.description,
    kind: toCanvasKind(s.kind),
    position: s.position,
  }));
  const byKey = new Set(states.map(s => s.key));
  const byName = new Map(states.map(s => [s.name, s.key]));

  const transitions: TransitionDraft[] = [];
  for (const t of payload.transitions ?? []) {
    // Models sometimes refer to states by name instead of id.
    const from = byKey.has(t.from) ? t.from : byName.get(t.from);
    const to   = byKey.has(t.to)   ? t.to   : byName.get(t.to);
    if (!from || !to) continue;
    transitions.push({
      from, to,
      event: t.event?.trim() || 'transition',
      guard: t.guard?.trim() || undefined,
      action: t.action?.trim() || undefined,
    });
  }
  return { states, transitions };
}
