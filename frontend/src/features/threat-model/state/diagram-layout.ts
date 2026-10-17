/**
 * The data-flow diagram's layout: where components and zone regions are and
 * how parallel flows curve. Presentation only, saved opaque with the threat
 * model (ADR 0011); it never affects crossings, threats or tests. Pure, for
 * tests.
 */
import { ArchComponent, Flow, Zone } from '../../../core/api/threat-model-api';
import {
  Anchor, ANCHORS, ElementStyle, NodeShape, STYLE_KEYS,
} from '../../models/state/model-editor.store';
import { ALL_SHAPES } from '../../models/state/node-shapes';
import { freeSpot, Rect, regionAt } from './threat-model-view';

export const NODE_W = 150;
export const NODE_H = 64;
/** Space between a zone's edge and the components inside it. */
export const ZONE_PAD = 28;

/** A component's box, and the user's shape and style (as in the model editor). */
export interface NodeLayout extends Rect {
  shape?: NodeShape;
  style?: ElementStyle;
}

/** A zone's region and the user's style. */
export interface ZoneLayout extends Rect {
  style?: ElementStyle;
}

/** How a flow is drawn: its connector dots, routing, bend and style. */
export interface EdgeLayout {
  fromAnchor?: Anchor;
  toAnchor?: Anchor;
  /** Perpendicular bend; parallel flows get distinct ones. */
  curve?: number;
  routing?: 'curved' | 'orthogonal';
  style?: ElementStyle;
}

export interface DiagramLayout {
  v: 1;
  nodes: Record<string, NodeLayout>;
  zones: Record<string, ZoneLayout>;
  edges: Record<string, EdgeLayout>;
}

export function emptyLayout(): DiagramLayout {
  return { v: 1, nodes: {}, zones: {}, edges: {} };
}

const SHAPES = new Set<string>(ALL_SHAPES.map(s => s.value));
const COLOUR = /^(#[0-9a-f]{3,8}|[a-z]+|rgba?\([\d\s.,%]+\))$/i;

/** The style keys and values the model editor knows; anything else is dropped. */
export function parseStyle(raw: unknown): ElementStyle | undefined {
  if (!raw || typeof raw !== 'object') return undefined;
  const o = raw as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const k of STYLE_KEYS) {
    const v = o[k];
    if (v === undefined) continue;
    const ok =
      k === 'stroke' || k === 'fill' || k === 'text' ? typeof v === 'string' && COLOUR.test(v)
      : k === 'dash' ? v === 'dashed' || v === 'dotted'
      : k === 'width' ? v === 1 || v === 2 || v === 3 || v === 4
      : k === 'size' ? v === 's' || v === 'l'
      : k === 'arrow' ? v === 'open' || v === 'line'
      : typeof v === 'boolean';
    if (ok) out[k] = v;
  }
  return Object.keys(out).length ? (out as ElementStyle) : undefined;
}

function isRect(r: unknown): r is Rect {
  const o = r as Rect;
  return !!o && [o.x, o.y, o.w, o.h].every(n => typeof n === 'number' && Number.isFinite(n)) && o.w > 0 && o.h > 0;
}

/** Reads a stored layout; anything it does not understand is ignored. */
export function parseLayout(raw: unknown): DiagramLayout {
  const out = emptyLayout();
  const o = raw as (Partial<DiagramLayout> & { curves?: Record<string, unknown> }) | null;
  if (!o || o.v !== 1) return out;
  for (const [id, r] of Object.entries(o.nodes ?? {})) {
    if (!isRect(r)) continue;
    const n: NodeLayout = { x: r.x, y: r.y, w: r.w, h: r.h };
    if (r.shape && SHAPES.has(r.shape)) n.shape = r.shape;
    const st = parseStyle(r.style);
    if (st) n.style = st;
    out.nodes[id] = n;
  }
  for (const [id, r] of Object.entries(o.zones ?? {})) {
    if (!isRect(r)) continue;
    const z: ZoneLayout = { x: r.x, y: r.y, w: r.w, h: r.h };
    const st = parseStyle(r.style);
    if (st) z.style = st;
    out.zones[id] = z;
  }
  // Layouts saved before flows had dots kept only their bend, under `curves`.
  for (const [id, c] of Object.entries(o.curves ?? {})) if (typeof c === 'number') out.edges[id] = { curve: c };
  for (const [id, e] of Object.entries(o.edges ?? {})) {
    if (!e || typeof e !== 'object') continue;
    const x: EdgeLayout = {};
    if (ANCHORS.includes(e.fromAnchor as Anchor)) x.fromAnchor = e.fromAnchor;
    if (ANCHORS.includes(e.toAnchor as Anchor)) x.toAnchor = e.toAnchor;
    if (typeof e.curve === 'number' && Number.isFinite(e.curve)) x.curve = e.curve;
    if (e.routing === 'orthogonal' || e.routing === 'curved') x.routing = e.routing;
    const st = parseStyle(e.style);
    if (st) x.style = st;
    out.edges[id] = x;
  }
  return out;
}

const union = (rs: Rect[]): Rect | null => {
  if (!rs.length) return null;
  const x = Math.min(...rs.map(r => r.x)), y = Math.min(...rs.map(r => r.y));
  return { x, y, w: Math.max(...rs.map(r => r.x + r.w)) - x, h: Math.max(...rs.map(r => r.y + r.h)) - y };
};

const pad = (r: Rect, p: number, top = p): Rect => ({ x: r.x - p, y: r.y - top, w: r.w + 2 * p, h: r.h + p + top });

const contains = (outer: Rect, inner: Rect) =>
  inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.w <= outer.x + outer.w && inner.y + inner.h <= outer.y + outer.h;

/** Smallest rect containing both. */
export function grow(outer: Rect, inner: Rect): Rect {
  return union([outer, inner])!;
}

/** A zone region fitted around the given member boxes. */
export function fitAround(members: Rect[]): Rect | null {
  const u = union(members);
  return u ? pad(u, ZONE_PAD, ZONE_PAD + 16) : null;
}

/**
 * The layout for the current entities: entries of deleted entities are
 * dropped, new components are placed without overlapping (inside their zone
 * when it has a region), zones get a region around their members and grow to
 * contain them, and parallel flows get distinct curves.
 */
export function resolveLayout(
  stored: DiagramLayout, components: ArchComponent[], zones: Zone[], flows: Flow[],
): DiagramLayout {
  const out = emptyLayout();
  for (const z of zones) if (stored.zones[z.id]) out.zones[z.id] = { ...stored.zones[z.id] };
  for (const c of components) if (stored.nodes[c.id]) out.nodes[c.id] = { ...stored.nodes[c.id] };

  // Zones without a region: around their placed members, else in free space.
  for (const z of zones) {
    if (out.zones[z.id]) continue;
    const members = components.filter(c => c.zoneId === z.id && out.nodes[c.id]).map(c => out.nodes[c.id]);
    const around = union(members);
    if (around) {
      out.zones[z.id] = pad(around, ZONE_PAD, ZONE_PAD + 16);
    } else {
      const spot = freeSpot(Object.values(out.zones), NODE_W + 2 * ZONE_PAD, NODE_H + 2 * ZONE_PAD + 16);
      out.zones[z.id] = { ...spot, w: NODE_W + 2 * ZONE_PAD, h: NODE_H + 2 * ZONE_PAD + 16 };
    }
  }

  // Components without a position: inside their zone, else outside all zones.
  for (const c of components) {
    if (out.nodes[c.id]) continue;
    const zone = c.zoneId ? out.zones[c.zoneId] : undefined;
    const taken = [...Object.values(out.nodes), ...(zone ? [] : Object.values(out.zones))];
    const spot = zone
      ? freeSpot(taken, NODE_W, NODE_H, zone)
      : freeSpot(taken, NODE_W, NODE_H);
    out.nodes[c.id] = { ...spot, w: NODE_W, h: NODE_H };
  }

  // A zone always contains its members.
  for (const c of components) {
    const z = c.zoneId ? out.zones[c.zoneId] : undefined;
    if (z && !contains(z, out.nodes[c.id])) {
      out.zones[c.zoneId!] = { ...grow(z, pad(out.nodes[c.id], ZONE_PAD, ZONE_PAD + 16)), style: z.style };
    }
  }

  // Parallel flows between the same pair fan out: 0, 40, -40, 80, …
  const seen = new Map<string, number>();
  for (const f of flows) {
    const e: EdgeLayout = { ...(stored.edges[f.id] ?? {}) };
    if (e.curve === undefined) {
      const key = [f.sourceId, f.targetId].sort().join('|');
      const i = seen.get(key) ?? 0;
      seen.set(key, i + 1);
      const mag = Math.ceil(i / 2) * 40;
      // The same pair in the other direction bends the other way already.
      e.curve = i % 2 ? mag : -mag;
    }
    out.edges[f.id] = e;
  }
  return out;
}

/** The zone a component dropped with its centre at `p` belongs to. */
export function zoneAt(p: { x: number; y: number }, layout: DiagramLayout): string | null {
  return regionAt(p, new Map(Object.entries(layout.zones)));
}

/**
 * Where the segment a→b leaves `r` (a inside r), for the trust-boundary
 * marker. Null when a is not inside or b is inside too.
 */
export function exitPoint(a: { x: number; y: number }, b: { x: number; y: number }, r: Rect): { x: number; y: number } | null {
  const inside = (p: { x: number; y: number }) => p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h;
  if (!inside(a) || inside(b)) return null;
  const dx = b.x - a.x, dy = b.y - a.y;
  let t = Infinity;
  if (dx > 0) t = Math.min(t, (r.x + r.w - a.x) / dx);
  if (dx < 0) t = Math.min(t, (r.x - a.x) / dx);
  if (dy > 0) t = Math.min(t, (r.y + r.h - a.y) / dy);
  if (dy < 0) t = Math.min(t, (r.y - a.y) / dy);
  return Number.isFinite(t) ? { x: a.x + dx * t, y: a.y + dy * t } : null;
}
