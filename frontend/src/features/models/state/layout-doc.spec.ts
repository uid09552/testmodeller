import { TestCase } from '../../../core/api/api.types';
import { layoutOf, overlayFromLayout } from './layout-doc';
import { fromRemote, PersistedModel, RemoteModel, toModelInput } from './model-mapping';
import { CanvasNode, SIZE_FOR_SHAPE } from './model-editor.store';

const A = '11111111-1111-4111-8111-111111111111';
const B = '22222222-2222-4222-8222-222222222222';
const C = '33333333-3333-4333-8333-333333333333';
const E = '44444444-4444-4444-8444-444444444444';
const M = '55555555-5555-4555-8555-555555555555';
const T = '66666666-6666-4666-8666-666666666666';

function node(partial: Partial<CanvasNode> & { id: string }): CanvasNode {
  return {
    label: 'S', kind: 'regular', x: 0, y: 0, w: 144, h: 48, shape: 'rect', color: null,
    tests: [], ...partial,
  };
}

/** What the editor holds after some layout work. */
function edited(): PersistedModel {
  return {
    id: M, name: 'Login', description: '', scenarioDesc: '', status: 'draft', testSeq: 8,
    variables: [{ name: 'attempts', type: 'integer', initial: 0 }],
    nodes: [
      node({ id: A, kind: 'initial', shape: 'circle', w: 88, h: 88, color: '#ff0000',
             tests: [{ id: T, seq: 7, name: 't', category: 'unit', polarity: 'positive',
                       given: '', when: '', then: '' }] }),
      node({ id: B, kind: 'decision', shape: 'diamond', w: 172, h: 104 }),
      node({ id: C, kind: 'final', shape: 'rect', w: 200, h: 60 }),
    ],
    edges: [{ id: E, fromId: A, toId: B, label: 'go', curve: 46, fromAnchor: 'right', toAnchor: 'left' }],
    groups: [{ id: 'g1', label: 'Auth', color: '#4f6ef2', opacity: 0.1, x: 0, y: 0, w: 300, h: 200 }],
  };
}

/** What `GET /models/{id}` returns after saving `m`. */
function remoteOf(m: PersistedModel): RemoteModel {
  const input = toModelInput(m);
  return { ...input, states: input.states!, transitions: input.transitions!, layout: input.layout ?? undefined };
}

const testCases: TestCase[] = [{
  id: T, version: 1, featureId: 'f', name: 't', steps: [], tags: ['unit', 'positive'],
  assignments: [{ modelId: M, stateId: A, testCaseId: T }],
}];

describe('layout document', () => {
  it('round-trips everything the contract cannot hold, through a save and a reload', () => {
    const m = edited();
    const back = fromRemote(remoteOf(m), testCases, M, '', null);

    const [a, b, c] = back.nodes;
    expect([a.shape, a.color, a.w, a.h]).toEqual(['circle', '#ff0000', 88, 88]);
    expect([b.kind, b.shape, b.w, b.h]).toEqual(['decision', 'diamond', 172, 104]);
    expect([c.kind, c.shape, c.w, c.h]).toEqual(['final', 'rect', 200, 60]);
    expect(back.edges[0]).toMatchObject({ curve: 46, fromAnchor: 'right', toAnchor: 'left' });
    expect(back.groups).toEqual(m.groups);
    expect(a.tests[0].seq).toBe(7);
    expect(back.testSeq).toBe(8);
  });

  it('stores only what differs from the defaults', () => {
    const plain: PersistedModel = {
      ...edited(), groups: [],
      nodes: [node({ id: A, kind: 'initial', shape: 'circle', ...SIZE_FOR_SHAPE.circle })],
      edges: [],
    };
    const doc = layoutOf(plain);
    expect(doc['v']).toBe(1);
    expect(doc['states']).toEqual({});
    expect(doc['transitions']).toEqual({});
  });

  it('shows defaults for a model without layout', () => {
    const m = edited();
    const back = fromRemote({ ...remoteOf(m), layout: undefined }, [], M, '', null);
    expect(back.nodes.map(n => n.shape)).toEqual(['circle', 'rect', 'rect']);
    expect(back.nodes[1].kind).toBe('regular');
    expect(back.edges[0].curve).toBe(0);
    expect(back.groups).toEqual([]);
  });

  it('falls back per element when the layout is partial', () => {
    const m = edited();
    const layout = { v: 1, states: { [A]: { color: '#00ff00' } } };
    const back = fromRemote({ ...remoteOf(m), layout }, [], M, '', null);
    // Only the colour is stored: the initial state keeps its default shape and size.
    expect([back.nodes[0].shape, back.nodes[0].color, back.nodes[0].w])
      .toEqual(['circle', '#00ff00', SIZE_FOR_SHAPE.circle.w]);
  });

  it('ignores entries for elements that no longer exist, and drops them on the next save', () => {
    const m = edited();
    const gone = '77777777-7777-4777-8777-777777777777';
    const layout = { v: 1, states: { [gone]: { shape: 'diamond' } }, transitions: { [gone]: { curve: 9 } } };
    const back = fromRemote({ ...remoteOf(m), layout }, [], M, '', null);
    expect(back.nodes.map(n => n.id)).toEqual([A, B, C]);
    const saved = layoutOf(back);
    expect(Object.keys(saved['states'] as object)).not.toContain(gone);
    expect(Object.keys(saved['transitions'] as object)).not.toContain(gone);
  });

  it('tolerates junk without failing to open the model', () => {
    expect(overlayFromLayout(null)).toBeNull();
    const overlay = overlayFromLayout({ v: 9, states: 'nope', transitions: [1], groups: [{ id: 3 }] });
    expect(overlay?.nodes).toEqual([]);
    expect(overlay?.groups).toEqual([]);
  });
});

describe('variables', () => {
  it('are sent back unchanged on every save', () => {
    const m = edited();
    const back = fromRemote(remoteOf(m), [], M, '', null);
    expect(back.variables).toEqual(m.variables);
    expect(toModelInput(back).variables).toEqual([{ name: 'attempts', type: 'integer', initial: 0 }]);
  });
});

describe('transition routing in the layout', () => {
  it('round-trips waypoints, routing style and label offset, and stores none of them by default', () => {
    const m = edited();
    m.edges = [{
      ...m.edges[0], waypoints: [{ x: 120, y: 200 }], routing: 'orthogonal', labelOffset: { dx: 4, dy: -30 },
    }];
    const back = fromRemote(remoteOf(m), [], M, '', null);
    expect(back.edges[0]).toMatchObject({
      waypoints: [{ x: 120, y: 200 }], routing: 'orthogonal', labelOffset: { dx: 4, dy: -30 },
    });

    const plain = edited();
    plain.edges = [{ ...plain.edges[0], curve: 0, fromAnchor: undefined, toAnchor: undefined }];
    expect(layoutOf(plain)['transitions']).toEqual({});
  });

  it('drops malformed routing data', () => {
    const overlay = overlayFromLayout({
      v: 1, transitions: { [E]: { waypoints: [{ x: 1 }, 'x', { x: 2, y: 3 }], routing: 'zigzag', labelOffset: { dx: 'a' } } },
    });
    expect(overlay!.edges[0].waypoints).toEqual([{ x: 2, y: 3 }]);
    expect(overlay!.edges[0].routing).toBeUndefined();
    expect(overlay!.edges[0].labelOffset).toBeUndefined();
  });
});

describe('collapsed groups in the layout', () => {
  it('saves and restores the collapsed state, and leaves the graph unchanged', () => {
    const m = edited();
    const open = toModelInput(m);
    m.groups = [{ ...m.groups![0], collapsed: true, members: [A, B] }];
    const collapsed = toModelInput(m);
    // Validation and generation see the same graph.
    expect({ ...collapsed, layout: null }).toEqual({ ...open, layout: null });

    const back = fromRemote(remoteOf(m), [], M, '', null);
    expect(back.groups![0]).toMatchObject({ collapsed: true, members: [A, B] });
  });
});

describe('transition expected result', () => {
  it('is kept through a load and a save, so the editor no longer deletes it', () => {
    const m = edited();
    m.edges = [{ ...m.edges[0], expected: 'the home screen is shown' }];
    const back = fromRemote(remoteOf(m), [], M, '', null);
    expect(back.edges[0].expected).toBe('the home screen is shown');
    expect(toModelInput(back).transitions![0].expected).toBe('the home screen is shown');
  });
});
