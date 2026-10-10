/**
 * The editor's layout document (ADR 0010): what the contract's graph cannot
 * hold, stored with the model as an opaque object the editor owns.
 *
 * Version 1:
 *   states[id]      { shape?, color?, w?, h?, decision?, style? }   only non-defaults
 *   transitions[id] { curve?, fromAnchor?, toAnchor?, waypoints?, routing?, labelOffset?, style? }
 *   groups          CanvasGroup[]
 *   annotations     { id, kind, x, y, w, h, text, style? }[]   notes and text boxes
 *   testSeq[id]     display number of a test case
 *   nextTestSeq     next display number
 *
 * `style` holds only the keys the user set (see `ElementStyle`). A state's
 * `color` is its line colour, written alongside `style.stroke` so that
 * editors from before `style` still show it.
 *
 * Readers keep what they understand and ignore the rest, so later versions
 * can add keys without breaking older editors.
 */
import { ModelLayoutDoc } from '../../../core/api/api.types';
import {
  ANNOTATION_MAX_TEXT, Anchor, ANCHORS, CanvasAnnotation, CanvasEdge, CanvasGroup, CanvasNode, ElementStyle, NodeShape, SHAPE_FOR_KIND,
  SIZE_FOR_SHAPE, StateTest,
} from './model-editor.store';
import type { PersistedModel } from './model-mapping';
import { ALL_SHAPES } from './node-shapes';

export const LAYOUT_VERSION = 1;

interface StateLayout {
  shape?: NodeShape; color?: string; w?: number; h?: number; decision?: boolean; style?: ElementStyle;
}
interface TransitionLayout {
  curve?: number; fromAnchor?: Anchor; toAnchor?: Anchor;
  waypoints?: { x: number; y: number }[]; routing?: 'orthogonal'; labelOffset?: { dx: number; dy: number };
  style?: ElementStyle;
}

const SHAPES: NodeShape[] = ALL_SHAPES.map(s => s.value);

/** Only what differs from the defaults, so a plain model stores almost nothing. */
export function layoutOf(m: PersistedModel): ModelLayoutDoc {
  const states: Record<string, StateLayout> = {};
  for (const n of m.nodes) {
    const kind = n.kind === 'decision' ? 'decision' : n.kind;
    const entry: StateLayout = {};
    if (n.shape !== SHAPE_FOR_KIND[kind]) entry.shape = n.shape;
    if (n.style) { entry.style = n.style; if (n.style.stroke) entry.color = n.style.stroke; }
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
    if (e.style) entry.style = e.style;
    if (Object.keys(entry).length) transitions[e.id] = entry;
  }
  const testSeq: Record<string, number> = {};
  for (const t of m.nodes.flatMap(n => n.tests)) testSeq[t.id] = t.seq;
  return {
    v: LAYOUT_VERSION,
    states,
    transitions,
    groups: m.groups ?? [],
    ...(m.annotations?.length ? { annotations: m.annotations } : {}),
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
const HEX_RE = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;
const colour = (v: unknown): string | undefined =>
  typeof v === 'string' && HEX_RE.test(v) ? v : undefined;
const oneOf = <T extends string | number>(v: unknown, allowed: readonly T[]): T | undefined =>
  allowed.includes(v as T) ? v as T : undefined;

/**
 * The style keys of `raw` this editor understands, with valid values;
 * anything else falls back to the default by being left out.
 */
export function styleFrom(raw: unknown, legacyStroke?: unknown): ElementStyle | undefined {
  const s = obj(raw);
  const style: ElementStyle = {
    stroke: colour(s['stroke']) ?? colour(legacyStroke),
    dash: oneOf(s['dash'], ['dashed', 'dotted'] as const),
    width: oneOf(s['width'], [1, 2, 3, 4] as const),
    fill: colour(s['fill']),
    text: colour(s['text']),
    size: oneOf(s['size'], ['s', 'l'] as const),
    bold: s['bold'] === true || undefined,
    italic: s['italic'] === true || undefined,
    arrow: oneOf(s['arrow'], ['open', 'line'] as const),
  };
  for (const k of Object.keys(style) as (keyof ElementStyle)[]) {
    if (style[k] === undefined) delete style[k];
  }
  return Object.keys(style).length ? style : undefined;
}

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
      style: styleFrom(s['style'], s['color']),
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
      ...(styleFrom(t['style']) ? { style: styleFrom(t['style']) } : {}),
    };
  });

  const groups = Array.isArray(layout['groups'])
    ? (layout['groups'] as unknown[]).map(obj).filter(g =>
        typeof g['id'] === 'string' && num(g['x']) !== undefined && num(g['y']) !== undefined
        && num(g['w']) !== undefined && num(g['h']) !== undefined) as unknown as CanvasGroup[]
    : [];

  // Annotations: drop any entry without an id, a known kind, a box or text.
  const annotations: CanvasAnnotation[] = Array.isArray(layout['annotations'])
    ? (layout['annotations'] as unknown[]).map(obj).flatMap(a => {
        const x = num(a['x']), y = num(a['y']), w = num(a['w']), h = num(a['h']);
        const kind = a['kind'] === 'note' || a['kind'] === 'text' ? a['kind'] : null;
        if (typeof a['id'] !== 'string' || !kind || typeof a['text'] !== 'string'
            || x === undefined || y === undefined || !w || !h) return [];
        const style = styleFrom(a['style']);
        return [{
          id: a['id'], kind, x, y, w, h, text: a['text'].slice(0, ANNOTATION_MAX_TEXT),
          ...(style ? { style } : {}),
        }];
      })
    : [];

  return {
    id: '', name: '', description: '', scenarioDesc: '', status: 'draft',
    nodes, edges, groups, annotations,
    testSeq: num(layout['nextTestSeq']) ?? 1,
  };
}
