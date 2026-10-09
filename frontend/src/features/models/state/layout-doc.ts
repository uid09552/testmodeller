/**
 * The editor's layout document (ADR 0010): what the contract's graph cannot
 * hold, stored with the model as an opaque object the editor owns.
 *
 * Version 1:
 *   states[id]      { shape?, color?, w?, h?, decision? }   only non-defaults
 *   transitions[id] { curve?, fromAnchor?, toAnchor?, waypoints?, routing?, labelOffset? }
 *   groups          CanvasGroup[]
 *   testSeq[id]     display number of a test case
 *   nextTestSeq     next display number
 *
 * Readers keep what they understand and ignore the rest, so later versions
 * can add keys without breaking older editors.
 */
import { ModelLayoutDoc } from '../../../core/api/api.types';
import {
  Anchor, ANCHORS, CanvasEdge, CanvasGroup, CanvasNode, NodeShape, SHAPE_FOR_KIND,
  SIZE_FOR_SHAPE, StateTest,
} from './model-editor.store';
import type { PersistedModel } from './model-mapping';

export const LAYOUT_VERSION = 1;

interface StateLayout { shape?: NodeShape; color?: string; w?: number; h?: number; decision?: boolean }
interface TransitionLayout {
  curve?: number; fromAnchor?: Anchor; toAnchor?: Anchor;
  waypoints?: { x: number; y: number }[]; routing?: 'orthogonal'; labelOffset?: { dx: number; dy: number };
}

const SHAPES: NodeShape[] = ['circle', 'rect', 'diamond'];

/** Only what differs from the defaults, so a plain model stores almost nothing. */
export function layoutOf(m: PersistedModel): ModelLayoutDoc {
  const states: Record<string, StateLayout> = {};
  for (const n of m.nodes) {
    const kind = n.kind === 'decision' ? 'decision' : n.kind;
    const entry: StateLayout = {};
    if (n.shape !== SHAPE_FOR_KIND[kind]) entry.shape = n.shape;
    if (n.color) entry.color = n.color;
    const size = SIZE_FOR_SHAPE[n.shape];
    if (n.w !== size.w || n.h !== size.h) { entry.w = Math.round(n.w); entry.h = Math.round(n.h); }
    if (n.kind === 'decision') entry.decision = true;
    if (Object.keys(entry).length) states[n.id] = entry;
  }
  const ids = new Set(m.nodes.map(n => n.id));
  const transitions: Record<string, TransitionLayout> = {};
  for (const e of m.edges) {
    if (!ids.has(e.fromId) || !ids.has(e.toId)) continue;
    const entry: TransitionLayout = {};
    if (e.curve) entry.curve = Math.round(e.curve);
    if (e.fromAnchor) entry.fromAnchor = e.fromAnchor;
    if (e.toAnchor) entry.toAnchor = e.toAnchor;
    if (e.waypoints?.length) entry.waypoints = e.waypoints;
    if (e.routing === 'orthogonal') entry.routing = 'orthogonal';
    if (e.labelOffset) entry.labelOffset = e.labelOffset;
    if (Object.keys(entry).length) transitions[e.id] = entry;
  }
  const testSeq: Record<string, number> = {};
  for (const t of m.nodes.flatMap(n => n.tests)) testSeq[t.id] = t.seq;
  return {
    v: LAYOUT_VERSION,
    states,
    transitions,
    groups: m.groups ?? [],
    testSeq,
    nextTestSeq: m.testSeq,
  };
}

const obj = (v: unknown): Record<string, unknown> =>
  v && typeof v === 'object' && !Array.isArray(v) ? v as Record<string, unknown> : {};
const num = (v: unknown): number | undefined =>
  typeof v === 'number' && Number.isFinite(v) ? v : undefined;
const anchor = (v: unknown): Anchor | undefined =>
  ANCHORS.includes(v as Anchor) ? v as Anchor : undefined;

/**
 * The stored layout as the in-memory overlay `fromRemote` merges: only the
 * fields it takes from a prior copy are meaningful. Null without a layout.
 */
export function overlayFromLayout(layout: ModelLayoutDoc | undefined | null): PersistedModel | null {
  if (!layout || typeof layout !== 'object') return null;
  const states = obj(layout['states']);
  const transitions = obj(layout['transitions']);
  const seqs = obj(layout['testSeq']);

  // Partial nodes: what is not stored stays undefined, so `fromRemote` falls
  // back to the defaults of the kind the database gives the state.
  const nodes = Object.entries(states).map(([id, raw]) => {
    const s = obj(raw);
    const w = num(s['w']);
    const h = num(s['h']);
    return {
      id,
      kind: s['decision'] === true ? 'decision' : 'regular',
      shape: SHAPES.includes(s['shape'] as NodeShape) ? s['shape'] as NodeShape : undefined,
      ...(w !== undefined && h !== undefined ? { w, h } : {}),
      color: typeof s['color'] === 'string' ? s['color'] : null,
      tests: [],
    } as unknown as CanvasNode;
  });
  // Display numbers ride on a holder that matches no state: `fromRemote`
  // looks tests up by id across all overlay nodes.
  const tests = Object.entries(seqs)
    .filter(([, v]) => num(v) !== undefined)
    .map(([id, v]) => ({ id, seq: v as number }) as StateTest);
  if (tests.length) nodes.push({ id: '', tests } as unknown as CanvasNode);

  const point = (v: unknown) => {
    const p = obj(v);
    const x = num(p['x']), y = num(p['y']);
    return x !== undefined && y !== undefined ? { x, y } : null;
  };
  const edges: CanvasEdge[] = Object.entries(transitions).map(([id, raw]) => {
    const t = obj(raw);
    const wps = Array.isArray(t['waypoints'])
      ? (t['waypoints'] as unknown[]).map(point).filter((p): p is { x: number; y: number } => !!p)
      : [];
    const off = obj(t['labelOffset']);
    const dx = num(off['dx']), dy = num(off['dy']);
    return {
      id, fromId: '', toId: '', label: '',
      curve: num(t['curve']) ?? 0,
      fromAnchor: anchor(t['fromAnchor']),
      toAnchor: anchor(t['toAnchor']),
      ...(wps.length ? { waypoints: wps } : {}),
      ...(t['routing'] === 'orthogonal' ? { routing: 'orthogonal' as const } : {}),
      ...(dx !== undefined && dy !== undefined ? { labelOffset: { dx, dy } } : {}),
    };
  });

  const groups = Array.isArray(layout['groups'])
    ? (layout['groups'] as unknown[]).map(obj).filter(g =>
        typeof g['id'] === 'string' && num(g['x']) !== undefined && num(g['y']) !== undefined
        && num(g['w']) !== undefined && num(g['h']) !== undefined) as unknown as CanvasGroup[]
    : [];

  return {
    id: '', name: '', description: '', scenarioDesc: '', status: 'draft',
    nodes, edges, groups,
    testSeq: num(layout['nextTestSeq']) ?? 1,
  };
}
