import { TestCase } from '../../../core/api/api.types';
import {
  fromApiStatus, fromRemote, isUuid, PersistedModel, remoteFingerprint, testFromApi, testToInput,
  toApiStatus, toModelInput, withUuids,
} from './model-mapping';
import { StateTest } from './model-editor.store';

const A = '11111111-1111-4111-8111-111111111111';
const B = '22222222-2222-4222-8222-222222222222';
const C = '33333333-3333-4333-8333-333333333333';
const E = '44444444-4444-4444-8444-444444444444';
const MODEL = '55555555-5555-4555-8555-555555555555';

function test(partial: Partial<StateTest> = {}): StateTest {
  return {
    id: '66666666-6666-4666-8666-666666666666', seq: 3, name: 'Valid login',
    category: 'integration', polarity: 'negative',
    given: 'the account is active\nthe page is open',
    when: 'credentials are submitted\nthe button is pressed',
    then: 'the home screen is shown',
    implementationUrl: 'https://github.com/x/y/login.spec.ts',
    backlogUrl: 'https://jira.example/TM-1',
    ...partial,
  };
}

function model(partial: Partial<PersistedModel> = {}): PersistedModel {
  return {
    id: 'local-1', name: 'Login Flow', description: 'Auth', scenarioDesc: 'As a user…',
    status: 'review', testSeq: 4,
    nodes: [
      { id: A, label: 'Start', kind: 'initial', x: 10, y: 20, w: 88, h: 88,
        shape: 'circle', color: '#22c55e', tests: [test()] },
      { id: B, label: 'Check', kind: 'decision', x: 200, y: 20, w: 172, h: 104,
        shape: 'diamond', color: null, tests: [] },
      { id: C, label: 'Home', kind: 'final', x: 400, y: 20, w: 144, h: 48,
        shape: 'rect', color: null, tests: [] },
    ],
    edges: [
      { id: E, fromId: A, toId: B, label: 'login', guard: 'ok', action: 'n := 1',
        curve: 24, fromAnchor: 'right', toAnchor: 'left' },
    ],
    groups: [{ id: 'g', label: 'Auth', color: '#4f6ef2', opacity: 0.1, x: 0, y: 0, w: 300, h: 200 }],
    ...partial,
  };
}

/** What the backend returns for a test case saved from `t`. */
function stored(t: StateTest, stateId: string): TestCase {
  const input = testToInput(t, MODEL, stateId);
  return {
    ...input,
    id: t.id, version: 1, featureId: 'f',
    assignments: [{ modelId: MODEL, stateId, testCaseId: t.id }],
  };
}

describe('withUuids', () => {
  it('rewrites ids that are not UUIDs and keeps the graph connected', () => {
    const m = withUuids(model({
      nodes: [
        { ...model().nodes[0], id: 'ds0', tests: [test({ id: 'dt1' })] },
        { ...model().nodes[1], id: 'ds1' },
      ],
      edges: [{ ...model().edges[0], id: 'de0', fromId: 'ds0', toId: 'ds1' }],
    }));

    expect(m.nodes.every(n => isUuid(n.id))).toBe(true);
    expect(isUuid(m.nodes[0].tests[0].id)).toBe(true);
    expect(isUuid(m.edges[0].id)).toBe(true);
    expect(m.edges[0].fromId).toBe(m.nodes[0].id);
    expect(m.edges[0].toId).toBe(m.nodes[1].id);
  });

  it('leaves valid ids alone, so the database keeps recognising them', () => {
    const m = withUuids(model());
    expect(m.nodes.map(n => n.id)).toEqual([A, B, C]);
    expect(m.edges[0].id).toBe(E);
  });
});

describe('status', () => {
  it('maps three statuses onto the contract s two', () => {
    // The backend rejects `approved` with a 400; this is why the mapping exists.
    expect(toApiStatus('approved')).toBe('ready');
    expect(toApiStatus('review')).toBe('draft');
    expect(toApiStatus('draft')).toBe('draft');
  });

  it('keeps `review` when the browser knows it, since the contract cannot', () => {
    expect(fromApiStatus('ready', 'review')).toBe('approved');
    expect(fromApiStatus('draft', 'review')).toBe('review');
    expect(fromApiStatus('draft', 'approved')).toBe('draft');
    expect(fromApiStatus('draft', undefined)).toBe('draft');
  });
});

describe('toModelInput', () => {
  it('sends the local ids, so assignments survive the save', () => {
    const input = toModelInput(model());
    expect(input.states?.map(s => s.id)).toEqual([A, B, C]);
    expect(input.transitions?.[0]).toMatchObject({ id: E, from: A, to: B, guard: 'ok' });
  });

  it('keeps line breaks in a state name through a save', () => {
    const m = model();
    m.nodes[2] = { ...m.nodes[2], label: 'Home\nscreen\nshown' };
    const input = toModelInput(m);
    expect(input.states?.[2].name).toBe('Home\nscreen\nshown');
  });

  it('sends a decision as a normal state', () => {
    expect(toModelInput(model()).states?.map(s => s.kind)).toEqual(['initial', 'normal', 'final']);
  });

  it('drops an edge to a missing state rather than failing the whole save', () => {
    const input = toModelInput(model({
      edges: [{ ...model().edges[0], toId: 'gone' }],
    }));
    expect(input.transitions).toEqual([]);
  });
});

describe('test cases', () => {
  it('carries category, polarity and links through a save and a load', () => {
    const original = test();
    const back = testFromApi(stored(original, A), original.seq);
    expect(back).toEqual(original);
  });

  it('pairs uneven When and Then lines into steps without losing any', () => {
    const input = testToInput(test({ when: 'a\nb\nc', then: 'x' }), MODEL, A);
    expect(input.steps).toEqual([
      { action: 'a', expected: 'x' },
      { action: 'b', expected: '' },
      { action: 'c', expected: '' },
    ]);
    const back = testFromApi({ ...stored(test(), A), steps: input.steps }, 1);
    expect(back.when).toBe('a\nb\nc');
    expect(back.then).toBe('x');
  });

  it('assigns the test case to its state in this model', () => {
    expect(testToInput(test(), MODEL, A).assignments).toEqual([{ modelId: MODEL, stateId: A }]);
  });

  it('omits empty optional fields', () => {
    const input = testToInput(
      test({ given: '', implementationUrl: '', backlogUrl: '' }), MODEL, A);
    expect(input.preconditions).toBeUndefined();
    expect(input.description).toBeUndefined();
    expect(input.implementationUrl).toBeUndefined();
    expect(input.backlogUrl).toBeUndefined();
  });

  it('sends the links as fields, not inside the description (FR-025)', () => {
    const input = testToInput(
      test({ implementationUrl: ' https://git/x ', backlogUrl: 'https://jira/TM-1' }), MODEL, A);
    expect(input.implementationUrl).toBe('https://git/x');
    expect(input.backlogUrl).toBe('https://jira/TM-1');
    expect(input.description).toBeUndefined();
  });

  it('reads the links from their fields', () => {
    const t = testFromApi({
      ...stored(test(), A),
      implementationUrl: 'https://git/x', backlogUrl: 'https://jira/TM-1',
      description: 'Backlog: https://old/ignored',
    }, 1);
    expect(t.implementationUrl).toBe('https://git/x');
    expect(t.backlogUrl).toBe('https://jira/TM-1');
  });

  it('falls back to the labelled description lines of data saved before the fields', () => {
    const t = testFromApi({
      ...stored(test({ implementationUrl: '', backlogUrl: '' }), A),
      description: 'Implementation: https://git/old\nBacklog: https://jira/OLD-1',
    }, 1);
    expect(t.implementationUrl).toBe('https://git/old');
    expect(t.backlogUrl).toBe('https://jira/OLD-1');
    // The next save writes them as fields and drops the lines.
    const input = testToInput(t, MODEL, A);
    expect(input.implementationUrl).toBe('https://git/old');
    expect(input.backlogUrl).toBe('https://jira/OLD-1');
    expect(input.description).toBeUndefined();
  });
});

describe('fromRemote', () => {
  it('round-trips a model: save, reload, and nothing on the canvas changes', () => {
    const local = model();
    const input = toModelInput(local);
    const reloaded = fromRemote(
      { name: input.name, description: input.description, status: input.status,
        states: input.states!, transitions: input.transitions! },
      [stored(local.nodes[0].tests[0], A)],
      MODEL, local.scenarioDesc, local,
    );

    expect({ ...reloaded, id: local.id }).toEqual(local);
  });

  it('lets the database win over the overlay for what it holds', () => {
    const local = model();
    const input = toModelInput(local);
    const states = input.states!.map(s => s.id === A ? { ...s, name: 'Renamed elsewhere' } : s);
    const reloaded = fromRemote(
      { ...input, states, transitions: input.transitions! }, [], MODEL, '', local);

    expect(reloaded.nodes[0].label).toBe('Renamed elsewhere');
    // …but keeps what only the overlay can know.
    expect(reloaded.nodes[0].color).toBe('#22c55e');
    expect(reloaded.nodes[0].shape).toBe('circle');
  });

  it('gives up a decision if the database turned it into an initial or final state', () => {
    const local = model();
    const input = toModelInput(local);
    const states = input.states!.map(s => s.id === B ? { ...s, kind: 'final' as const } : s);
    const reloaded = fromRemote(
      { ...input, states, transitions: input.transitions! }, [], MODEL, '', local);

    expect(reloaded.nodes[1].kind).toBe('final');
  });

  it('builds a model the browser has never seen, with default presentation', () => {
    const input = toModelInput(model());
    const reloaded = fromRemote(
      { ...input, states: input.states!, transitions: input.transitions! }, [], MODEL, '', null);

    expect(reloaded.nodes[1].kind).toBe('regular');
    expect(reloaded.nodes[1].shape).toBe('rect');
    expect(reloaded.edges[0].curve).toBe(0);
    expect(reloaded.groups).toEqual([]);
  });

  it('numbers test cases the browser has not seen after the ones it has', () => {
    const local = model();
    const input = toModelInput(local);
    const fresh = stored(test({ id: '77777777-7777-4777-8777-777777777777', seq: 0 }), A);
    const reloaded = fromRemote(
      { ...input, states: input.states!, transitions: input.transitions! },
      [stored(local.nodes[0].tests[0], A), fresh], MODEL, '', local);

    expect(reloaded.nodes[0].tests.map(t => t.seq)).toEqual([3, 4]);
    expect(reloaded.testSeq).toBe(5);
  });
});

describe('remoteFingerprint', () => {
  it('changes when something the database holds changes', () => {
    const before = remoteFingerprint(model());
    expect(remoteFingerprint(model({ name: 'Other' }))).not.toBe(before);
    expect(remoteFingerprint(model({ scenarioDesc: 'changed' }))).not.toBe(before);
  });

  it('ignores presentation, so restyling does not call the API', () => {
    const before = remoteFingerprint(model());
    const restyled = model();
    restyled.nodes[0] = { ...restyled.nodes[0], color: '#ef4444', w: 120 };
    restyled.edges[0] = { ...restyled.edges[0], curve: 80 };
    expect(remoteFingerprint(restyled)).toBe(before);
  });
});
