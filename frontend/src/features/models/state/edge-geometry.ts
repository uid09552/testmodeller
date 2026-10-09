/**
 * Where a transition runs on the canvas: its SVG path, its label point and
 * the positions of its handles. Pure, so every routing style can be tested
 * without a DOM. See docs/specification/06-ui.md#transition-routing.
 *
 * Styles:
 * - straight / curved: one perpendicular bend (`curve`), a quadratic Bezier;
 * - curved with waypoints: a centripetal Catmull-Rom spline through them;
 * - right-angle: horizontal and vertical segments, leaving and entering the
 *   states perpendicular to the side of their connector dots;
 * - self-loop: drawn outward on the side of its connector dot, larger for
 *   each further loop on the same side.
 */
import { Anchor, anchorPoint, CanvasEdge, CanvasNode } from './model-editor.store';

export interface Pt { x: number; y: number }

export interface EdgeGeometry {
  /** SVG path data. */
  d: string;
  /** Where the label sits, after the user's offset. */
  label: Pt;
  /** Where the label would sit without an offset (for the leader line). */
  labelHome: Pt;
  /** The bend handle, for a transition without waypoints that is not right-angle. */
  bend: Pt | null;
  /** End points on the states. */
  src: Pt;
  tgt: Pt;
}

/** Space between a state's outline and the start of a right-angle route. */
export const STUB = 16;
/** How far a label must be moved before a leader line joins it to its transition. */
export const LEADER_MIN = 12;

const mid = (a: Pt, b: Pt): Pt => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });
const centre = (n: CanvasNode): Pt => ({ x: n.x + n.w / 2, y: n.y + n.h / 2 });
const f = (n: number) => Math.round(n * 10) / 10;

/** Point on a state's outline in the direction of `toward`. */
export function borderPt(n: CanvasNode, toward: Pt): Pt {
  const cx = n.x + n.w / 2, cy = n.y + n.h / 2;
  const dx = toward.x - cx, dy = toward.y - cy;
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 0.001) return { x: cx, y: cy - n.h / 2 };
  const ndx = dx / len, ndy = dy / len;

  if (n.shape === 'circle') {
    const rx = n.w / 2 + 2, ry = n.h / 2 + 2;
    const k = 1 / Math.sqrt((ndx * ndx) / (rx * rx) + (ndy * ndy) / (ry * ry));
    return { x: cx + ndx * k, y: cy + ndy * k };
  }
  if (n.shape === 'diamond') {
    const a = n.w / 2 + 2, b = n.h / 2 + 2;
    const k = 1 / (Math.abs(ndx) / a + Math.abs(ndy) / b);
    return { x: cx + ndx * k, y: cy + ndy * k };
  }
  const hw = n.w / 2 + 2, hh = n.h / 2 + 2;
  let t = Infinity;
  if (Math.abs(ndx) > 0.001) {
    const tt = (ndx > 0 ? hw : -hw) / ndx;
    if (tt > 0 && Math.abs(cy + tt * ndy - cy) <= hh) t = Math.min(t, tt);
  }
  if (Math.abs(ndy) > 0.001) {
    const tt = (ndy > 0 ? hh : -hh) / ndy;
    if (tt > 0 && Math.abs(cx + tt * ndx - cx) <= hw) t = Math.min(t, tt);
  }
  if (!isFinite(t)) return { x: cx, y: cy };
  return { x: cx + ndx * t, y: cy + ndy * t };
}

/** The side of `n` that faces `toward`, by the dominant axis. */
export function facingSide(n: CanvasNode, toward: Pt): Anchor {
  const c = centre(n);
  const dx = toward.x - c.x, dy = toward.y - c.y;
  if (Math.abs(dx) * n.h >= Math.abs(dy) * n.w) return dx >= 0 ? 'right' : 'left';
  return dy >= 0 ? 'bottom' : 'top';
}

const NORMAL: Record<Anchor, Pt> = {
  top: { x: 0, y: -1 }, right: { x: 1, y: 0 }, bottom: { x: 0, y: 1 }, left: { x: -1, y: 0 },
};

/** Total length of a polyline and the point at fraction `t` of it. */
export function along(points: Pt[], t: number): Pt {
  const seg: number[] = [];
  let total = 0;
  for (let i = 1; i < points.length; i++) {
    const l = Math.hypot(points[i].x - points[i - 1].x, points[i].y - points[i - 1].y);
    seg.push(l);
    total += l;
  }
  if (total === 0) return points[0];
  let want = total * t;
  for (let i = 0; i < seg.length; i++) {
    if (want <= seg[i] || i === seg.length - 1) {
      const k = seg[i] ? Math.min(1, want / seg[i]) : 0;
      const a = points[i], b = points[i + 1];
      return { x: a.x + (b.x - a.x) * k, y: a.y + (b.y - a.y) * k };
    }
    want -= seg[i];
  }
  return points[points.length - 1];
}

/** Unit normal of the chord a -> b (the direction a positive `curve` bends to). */
function chordNormal(a: Pt, b: Pt): Pt {
  const pdx = b.y - a.y, pdy = -(b.x - a.x);
  const l = Math.hypot(pdx, pdy) || 1;
  return { x: pdx / l, y: pdy / l };
}

/**
 * The `curve` that makes the quadratic from `a` to `b` pass through `p`'s
 * projection onto the chord's perpendicular: the curve's midpoint sits at
 * half the control offset, so the offset is twice the distance.
 */
export function curveThrough(a: Pt, b: Pt, p: Pt): number {
  const m = mid(a, b);
  const n = chordNormal(a, b);
  return Math.round(2 * ((p.x - m.x) * n.x + (p.y - m.y) * n.y));
}

/** Index at which a waypoint at `p` belongs: on the nearest segment of the route. */
export function insertIndex(src: Pt, waypoints: Pt[], tgt: Pt, p: Pt): number {
  const pts = [src, ...waypoints, tgt];
  let best = 0, bestD = Infinity;
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1], b = pts[i];
    const dx = b.x - a.x, dy = b.y - a.y;
    const l2 = dx * dx + dy * dy || 1;
    const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2));
    const d = Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
    if (d < bestD) { bestD = d; best = i - 1; }
  }
  return best;
}

/**
 * Centripetal Catmull-Rom (alpha 0.5) through `pts` as cubic Bezier segments,
 * which never loops or cusps between close points. Control points after
 * Yuksel et al.; an end segment falls back to a third of the chord.
 */
function spline(pts: Pt[]): string {
  const dist = (a: Pt, b: Pt) => Math.hypot(b.x - a.x, b.y - a.y);
  let d = `M ${f(pts[0].x)} ${f(pts[0].y)}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p1 = pts[i], p2 = pts[i + 1];
    const p0 = pts[i - 1], p3 = pts[i + 2];
    const l12 = dist(p1, p2), r12 = Math.sqrt(l12);
    let c1: Pt = { x: p1.x + (p2.x - p1.x) / 3, y: p1.y + (p2.y - p1.y) / 3 };
    let c2: Pt = { x: p2.x - (p2.x - p1.x) / 3, y: p2.y - (p2.y - p1.y) / 3 };
    if (p0 && l12 > 0) {
      const l01 = dist(p0, p1), r01 = Math.sqrt(l01);
      if (l01 > 0) {
        const k = 3 * r01 * (r01 + r12);
        const w = 2 * l01 + 3 * r01 * r12 + l12;
        c1 = { x: (l01 * p2.x - l12 * p0.x + w * p1.x) / k, y: (l01 * p2.y - l12 * p0.y + w * p1.y) / k };
      }
    }
    if (p3 && l12 > 0) {
      const l23 = dist(p2, p3), r23 = Math.sqrt(l23);
      if (l23 > 0) {
        const k = 3 * r23 * (r23 + r12);
        const w = 2 * l23 + 3 * r23 * r12 + l12;
        c2 = { x: (l23 * p1.x - l12 * p3.x + w * p2.x) / k, y: (l23 * p1.y - l12 * p3.y + w * p2.y) / k };
      }
    }
    d += ` C ${f(c1.x)} ${f(c1.y)} ${f(c2.x)} ${f(c2.y)} ${f(p2.x)} ${f(p2.y)}`;
  }
  return d;
}

/** Right-angle route: stubs off both sides, elbows between. */
export function orthogonalPoints(src: Pt, srcSide: Anchor, tgt: Pt, tgtSide: Anchor, waypoints: Pt[]): Pt[] {
  const s1 = { x: src.x + NORMAL[srcSide].x * STUB, y: src.y + NORMAL[srcSide].y * STUB };
  const t1 = { x: tgt.x + NORMAL[tgtSide].x * STUB, y: tgt.y + NORMAL[tgtSide].y * STUB };
  const route: Pt[] = [src, s1];
  let horizontal = srcSide === 'left' || srcSide === 'right';
  for (const q of [...waypoints, t1]) {
    const p = route[route.length - 1];
    if (p.x !== q.x && p.y !== q.y) {
      route.push(horizontal ? { x: q.x, y: p.y } : { x: p.x, y: q.y });
    } else {
      horizontal = p.y === q.y;
    }
    route.push(q);
  }
  route.push(tgt);
  // Drop repeated points and collinear middles.
  return route.filter((p, i) => {
    const a = route[i - 1], b = route[i + 1];
    if (a && a.x === p.x && a.y === p.y) return false;
    return !(a && b && ((a.x === p.x && p.x === b.x) || (a.y === p.y && p.y === b.y)));
  });
}

function offsetLabel(e: CanvasEdge, home: Pt): Pt {
  const o = e.labelOffset;
  return o ? { x: home.x + o.dx, y: home.y + o.dy } : home;
}

/** Self-loop on the side of its connector dot; `index` grows each further loop on that side. */
function selfLoop(e: CanvasEdge, n: CanvasNode, index: number): EdgeGeometry {
  const side: Anchor = e.fromAnchor ?? 'top';
  const base = anchorPoint(n, side);
  const nrm = NORMAL[side];
  const along = { x: -nrm.y, y: nrm.x }; // tangent to the side
  const scale = 1 + 0.45 * index;
  const spread = 18 * scale, reach = 72 * scale, wide = 55 * scale;
  const out = (p: Pt, k: number) => ({ x: p.x + nrm.x * k, y: p.y + nrm.y * k });
  const at = (k: number) => ({ x: base.x + along.x * k, y: base.y + along.y * k });
  const a = out(at(-spread), 2), b = out(at(spread), 2);
  const c1 = out(at(-wide), reach), c2 = out(at(wide), reach);
  const labelHome = out(base, reach * 0.75 + 16);
  return {
    d: `M ${f(a.x)} ${f(a.y)} C ${f(c1.x)} ${f(c1.y)} ${f(c2.x)} ${f(c2.y)} ${f(b.x)} ${f(b.y)}`,
    label: offsetLabel(e, labelHome), labelHome, bend: null, src: a, tgt: b,
  };
}

/**
 * Geometry of a transition. `loopIndex` is this loop's position among the
 * self-loops on the same side of the same state.
 */
export function edgeGeometry(
  e: CanvasEdge, from: CanvasNode, to: CanvasNode, loopIndex = 0,
): EdgeGeometry {
  if (from.id === to.id) return selfLoop(e, from, loopIndex);
  const wps = e.waypoints ?? [];
  const firstToward = wps[0] ?? centre(to);
  const lastToward = wps[wps.length - 1] ?? centre(from);

  if (e.routing === 'orthogonal') {
    const srcSide = e.fromAnchor ?? facingSide(from, firstToward);
    const tgtSide = e.toAnchor ?? facingSide(to, lastToward);
    const src = anchorPoint(from, srcSide), tgt = anchorPoint(to, tgtSide);
    const pts = orthogonalPoints(src, srcSide, tgt, tgtSide, wps);
    const home = along(pts, 0.5);
    return {
      d: pts.map((p, i) => `${i ? 'L' : 'M'} ${f(p.x)} ${f(p.y)}`).join(' '),
      label: offsetLabel(e, home), labelHome: home, bend: null, src, tgt,
    };
  }

  const src = e.fromAnchor ? anchorPoint(from, e.fromAnchor) : borderPt(from, firstToward);
  const tgt = e.toAnchor ? anchorPoint(to, e.toAnchor) : borderPt(to, lastToward);

  if (wps.length) {
    const pts = [src, ...wps, tgt];
    const home = along(pts, 0.5);
    return { d: spline(pts), label: offsetLabel(e, home), labelHome: home, bend: null, src, tgt };
  }

  const m = mid(src, tgt);
  if (!e.curve) {
    return {
      d: `M ${f(src.x)} ${f(src.y)} L ${f(tgt.x)} ${f(tgt.y)}`,
      label: offsetLabel(e, m), labelHome: m, bend: m, src, tgt,
    };
  }
  const n = chordNormal(src, tgt);
  const cp = { x: m.x + n.x * e.curve, y: m.y + n.y * e.curve };
  const onCurve = { x: m.x + n.x * e.curve / 2, y: m.y + n.y * e.curve / 2 };
  return {
    d: `M ${f(src.x)} ${f(src.y)} Q ${f(cp.x)} ${f(cp.y)} ${f(tgt.x)} ${f(tgt.y)}`,
    label: offsetLabel(e, onCurve), labelHome: onCurve, bend: onCurve, src, tgt,
  };
}

/** Index of each self-loop among the loops on the same side of its state, by edge id. */
export function loopIndices(edges: CanvasEdge[]): Map<string, number> {
  const seen = new Map<string, number>();
  const out = new Map<string, number>();
  for (const e of edges) {
    if (e.fromId !== e.toId) continue;
    const key = `${e.fromId}:${e.fromAnchor ?? 'top'}`;
    const i = seen.get(key) ?? 0;
    out.set(e.id, i);
    seen.set(key, i + 1);
  }
  return out;
}

/** True when the label is far enough from its home to need a leader line. */
export function needsLeader(g: EdgeGeometry): boolean {
  return Math.hypot(g.label.x - g.labelHome.x, g.label.y - g.labelHome.y) > LEADER_MIN;
}
