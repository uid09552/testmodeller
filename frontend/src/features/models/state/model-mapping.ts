/**
 * Between the editor's model and the contract's (ADR 0008).
 *
 * The database holds what the contract can express: states, transitions,
 * positions, guards and actions, the model's name, description and status,
 * and test cases assigned to states. The canvas holds more — shapes, colours,
 * sizes, groups, edge curves, the `decision` kind, a three-way status, test
 * links and display numbers — and that is not stored: after a reload a state
 * has the default shape and colour for its kind. `fromRemote` can still merge
 * a copy held in memory, by id.
 *
 * Pure, so the round trip can be tested without a backend.
 */
import {
  ApiModelStatus, ModelInput, StateInput, TestCase, TestCaseInput, TransitionInput,
} from '../../../core/api/api.types';
import {
  CanvasEdge, CanvasGroup, CanvasNode, ModelStatus, SHAPE_FOR_KIND, SIZE_FOR_SHAPE, StateKind,
  StateTest, TestCategory,
} from './model-editor.store';

/** The full content of one model, as the editor holds it. */
export interface PersistedModel {
  id: string;
  name: string;
  description: string;
  scenarioDesc: string;
  status: ModelStatus;
  nodes: CanvasNode[];
  edges: CanvasEdge[];
  groups?: CanvasGroup[];
  /** Next test-id counter, so ids are never reused across sessions. */
  testSeq: number;
}

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function isUuid(id: string): boolean {
  return UUID_RE.test(id);
}

/**
 * Gives every state, transition and test a UUID.
 *
 * The backend keeps the ids a client supplies, and keeps a test case's
 * assignment as long as its state's id survives a save. So local ids must be
 * the database's ids — and models made before this (the demo ones, `ds0`)
 * have ids that are not UUIDs. Rewritten once; already-valid ids are kept.
 */
export function withUuids(m: PersistedModel): PersistedModel {
  const remap = new Map<string, string>();
  const fix = (id: string) => {
    if (isUuid(id)) return id;
    let next = remap.get(id);
    if (!next) {
      next = crypto.randomUUID();
      remap.set(id, next);
    }
    return next;
  };
  return {
    ...m,
    nodes: m.nodes.map(n => ({
      ...n,
      id: fix(n.id),
      tests: n.tests.map(t => ({ ...t, id: fix(t.id) })),
    })),
    edges: m.edges.map(e => ({ ...e, id: fix(e.id), fromId: fix(e.fromId), toId: fix(e.toId) })),
  };
}

// ── Status ───────────────────────────────────────────────────────────────────

/** The editor's three statuses onto the contract's two. */
export function toApiStatus(status: ModelStatus): ApiModelStatus {
  return status === 'approved' ? 'ready' : 'draft';
}

/**
 * Back again. `ready` is approved; `draft` is draft, unless the browser knows
 * it was `review`, which the contract cannot say.
 */
export function fromApiStatus(
  status: ApiModelStatus | string | undefined, local?: ModelStatus,
): ModelStatus {
  if (status === 'ready') return 'approved';
  return local === 'review' ? 'review' : 'draft';
}

// ── Graph ────────────────────────────────────────────────────────────────────

function toApiKind(kind: StateKind): StateInput['kind'] {
  return kind === 'initial' ? 'initial' : kind === 'final' ? 'final' : 'normal';
}

/** The model's graph and metadata, in the shape `PUT /models/{id}` takes. */
export function toModelInput(m: PersistedModel): ModelInput {
  const ids = new Set(m.nodes.map(n => n.id));
  const states: StateInput[] = m.nodes.map(n => ({
    id: n.id,
    name: n.label?.trim() || 'State',
    description: n.description?.trim() || undefined,
    kind: toApiKind(n.kind),
    position: { x: Math.round(n.x), y: Math.round(n.y) },
  }));
  const transitions: TransitionInput[] = m.edges
    // An edge to a state that is gone would make the whole save fail as a
    // structurally broken graph.
    .filter(e => ids.has(e.fromId) && ids.has(e.toId))
    .map(e => ({
      id: e.id,
      from: e.fromId,
      to: e.toId,
      event: e.label?.trim() || 'transition',
      guard: e.guard?.trim() || undefined,
      action: e.action?.trim() || undefined,
    }));
  return {
    name: m.name?.trim() || 'Model',
    description: m.description?.trim() || undefined,
    status: toApiStatus(m.status),
    states,
    transitions,
  };
}

// ── Test cases ───────────────────────────────────────────────────────────────

const CATEGORIES: TestCategory[] = ['unit', 'integration', 'feature'];
/** Labels earlier versions wrote the links under in the description (read as a fallback only). */
const LEGACY_IMPL = 'Implementation: ';
const LEGACY_BACKLOG = 'Backlog: ';

function lines(text: string | undefined): string[] {
  return (text ?? '').split('\n').map(l => l.trim()).filter(Boolean);
}

/**
 * A canvas test case as a stored one, assigned to its state.
 *
 * Given is the precondition; When and Then lines pair up into steps, by
 * position. Category and polarity travel as tags, which also makes them
 * filterable and export as Gherkin tags. The two links are fields of their
 * own.
 */
export function testToInput(t: StateTest, modelId: string, stateId: string): TestCaseInput {
  const when = lines(t.when);
  const then = lines(t.then);
  const steps = Array.from({ length: Math.max(when.length, then.length) }, (_, i) => ({
    action: when[i] ?? '',
    expected: then[i] ?? '',
  }));
  return {
    name: t.name?.trim() || 'Test case',
    preconditions: lines(t.given).join('\n') || undefined,
    implementationUrl: t.implementationUrl?.trim() || undefined,
    backlogUrl: t.backlogUrl?.trim() || undefined,
    tags: [t.category, t.polarity],
    steps,
    assignments: [{ modelId, stateId }],
  };
}

/** A stored test case back onto the canvas. `seq` is display-only and local. */
export function testFromApi(tc: TestCase, seq: number): StateTest {
  const tags = (tc.tags ?? []).map(t => t.toLowerCase());
  const descLines = (tc.description ?? '').split('\n');
  // Data saved before the links had fields still has them in the description.
  const legacy = (prefix: string) =>
    descLines.find(l => l.startsWith(prefix))?.slice(prefix.length).trim() ?? '';
  return {
    id: tc.id,
    seq,
    name: tc.name,
    category: CATEGORIES.find(c => tags.includes(c)) ?? 'feature',
    polarity: tags.includes('negative') ? 'negative' : 'positive',
    given: tc.preconditions ?? '',
    when: tc.steps.map(s => s.action?.trim()).filter(Boolean).join('\n'),
    then: tc.steps.map(s => s.expected?.trim()).filter(Boolean).join('\n'),
    implementationUrl: tc.implementationUrl ?? legacy(LEGACY_IMPL),
    backlogUrl: tc.backlogUrl ?? legacy(LEGACY_BACKLOG),
  };
}

// ── Loading ──────────────────────────────────────────────────────────────────

/** What `GET /models/{id}` returns, as far as loading needs. */
export interface RemoteModel {
  name: string;
  description?: string;
  status?: string;
  states: StateInput[];
  transitions: TransitionInput[];
}

/**
 * A stored model, with the browser's overlay merged in by id.
 *
 * The database wins for everything it holds. The overlay contributes only
 * what the contract cannot carry: a state keeps its shape, colour and size; a
 * `normal` state the browser knows to be a decision stays one; a transition
 * keeps its curve and connector dots; groups and test numbers come along.
 * Anything the browser has that the database does not is dropped — the
 * database is the copy that is shared.
 */
export function fromRemote(
  remote: RemoteModel,
  testCases: TestCase[],
  modelId: string,
  scenarioDesc: string,
  local: PersistedModel | null,
): PersistedModel {
  const localNodes = new Map((local?.nodes ?? []).map(n => [n.id, n]));
  const localEdges = new Map((local?.edges ?? []).map(e => [e.id, e]));
  const localTests = new Map(
    (local?.nodes ?? []).flatMap(n => n.tests).map(t => [t.id, t]));

  let nextSeq = local?.testSeq ?? 1;
  const seqFor = (id: string) => {
    const known = localTests.get(id)?.seq;
    if (known !== undefined) return known;
    return nextSeq++;
  };

  const testsByState = new Map<string, StateTest[]>();
  for (const tc of testCases) {
    const assignment = tc.assignments.find(a => a.modelId === modelId && a.stateId);
    if (!assignment?.stateId) continue;
    const list = testsByState.get(assignment.stateId) ?? [];
    list.push(testFromApi(tc, seqFor(tc.id)));
    testsByState.set(assignment.stateId, list);
  }

  const nodes: CanvasNode[] = remote.states.map((s, i) => {
    const id = s.id ?? crypto.randomUUID();
    const prior = localNodes.get(id);
    const apiKind: StateKind = s.kind === 'initial' ? 'initial'
      : s.kind === 'final' ? 'final' : 'regular';
    // `decision` survives only while the database still calls it `normal`:
    // a change to initial or final made elsewhere wins.
    const kind: StateKind = apiKind === 'regular' && prior?.kind === 'decision'
      ? 'decision' : apiKind;
    const shape = prior?.shape ?? SHAPE_FOR_KIND[kind];
    const size = prior ? { w: prior.w, h: prior.h } : SIZE_FOR_SHAPE[shape];
    return {
      id,
      label: s.name,
      kind,
      x: s.position?.x ?? 80 + (i % 4) * 200,
      y: s.position?.y ?? 80 + Math.floor(i / 4) * 140,
      ...size,
      shape,
      color: prior?.color ?? null,
      description: s.description,
      tests: (testsByState.get(id) ?? []).sort((a, b) => a.seq - b.seq),
    };
  });

  const edges: CanvasEdge[] = remote.transitions.map(t => {
    const id = t.id ?? crypto.randomUUID();
    const prior = localEdges.get(id);
    return {
      id,
      fromId: t.from,
      toId: t.to,
      label: t.event,
      guard: t.guard,
      action: t.action,
      curve: prior?.curve ?? 0,
      fromAnchor: prior?.fromAnchor,
      toAnchor: prior?.toAnchor,
    };
  });

  const maxSeq = nodes.flatMap(n => n.tests).reduce((m, t) => Math.max(m, t.seq), 0);
  return {
    id: local?.id ?? '',
    name: remote.name,
    description: remote.description ?? '',
    scenarioDesc,
    status: fromApiStatus(remote.status, local?.status),
    nodes,
    edges,
    groups: local?.groups ?? [],
    testSeq: Math.max(nextSeq, maxSeq + 1),
  };
}

/**
 * A stable fingerprint of what a save would send, so a save that would change
 * nothing is skipped. Only covers what reaches the database: moving a group
 * or recolouring a state is not a reason to call the API.
 */
export function remoteFingerprint(m: PersistedModel): string {
  const tests = m.nodes.flatMap(n =>
    n.tests.map(t => [n.id, t.id, testToInput(t, '', n.id)]));
  return JSON.stringify([toModelInput(m), m.scenarioDesc ?? '', tests]);
}
