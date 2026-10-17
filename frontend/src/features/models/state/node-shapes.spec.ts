import { anchorPoint, ANCHORS, CanvasNode, NodeShape, SIZE_FOR_SHAPE } from './model-editor.store';
import { ALL_SHAPES, anchorInset, capRy, shapePath, slant, wave } from './node-shapes';

/** All x,y pairs in a path, control points included. */
function points(d: string): { x: number; y: number }[] {
  const nums = d.replace(/[A-Z]/g, ' ').trim().split(/[\s,]+/).map(Number);
  // Arc commands carry rx, ry, rotation and two flags before their end point.
  const out: { x: number; y: number }[] = [];
  const tokens = d.match(/[A-Z][^A-Z]*/g)!;
  for (const t of tokens) {
    const n = t.slice(1).trim().split(/[\s,]+/).filter(Boolean).map(Number);
    if (t[0] === 'A') out.push({ x: n[5], y: n[6] });
    else for (let i = 0; i + 1 < n.length; i += 2) out.push({ x: n[i], y: n[i + 1] });
  }
  expect(nums.every(Number.isFinite)).toBe(true);
  return out;
}

/** True when p lies on the segment a–b. */
function onSegment(p: { x: number; y: number }, a: { x: number; y: number }, b: { x: number; y: number }): boolean {
  const cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
  const within = p.x >= Math.min(a.x, b.x) - 1e-6 && p.x <= Math.max(a.x, b.x) + 1e-6
              && p.y >= Math.min(a.y, b.y) - 1e-6 && p.y <= Math.max(a.y, b.y) + 1e-6;
  return Math.abs(cross) < 1e-3 && within;
}

const node = (shape: NodeShape): CanvasNode => ({
  id: 's', label: 's', kind: 'regular', x: 100, y: 50, ...SIZE_FOR_SHAPE[shape], shape, tests: [],
});

describe('node shapes', () => {
  it('offers seven shapes, each with a default size', () => {
    expect(ALL_SHAPES.map(s => s.value))
      .toEqual(['circle', 'rect', 'diamond', 'hexagon', 'parallelogram', 'cylinder', 'document']);
    for (const s of ALL_SHAPES) expect(SIZE_FOR_SHAPE[s.value].w).toBeGreaterThan(0);
  });

  it('keeps every outline point inside its box, except a document wave control point', () => {
    for (const { value } of ALL_SHAPES) {
      const { w, h } = SIZE_FOR_SHAPE[value];
      const pts = points(shapePath(value, w, h));
      const limitY = value === 'document' ? h + wave(h) : h;
      for (const p of pts) {
        expect(p.x).toBeGreaterThanOrEqual(0);
        expect(p.x).toBeLessThanOrEqual(w);
        expect(p.y).toBeGreaterThanOrEqual(0);
        expect(p.y).toBeLessThanOrEqual(limitY);
      }
    }
  });

  it('puts the connector points of a hexagon on its outline', () => {
    const n = node('hexagon');
    const k = slant(n.w, n.h);
    const at = (a: typeof ANCHORS[number]) => { const p = anchorPoint(n, a); return { x: p.x - n.x, y: p.y - n.y }; };
    expect(onSegment(at('top'), { x: k, y: 0 }, { x: n.w - k, y: 0 })).toBe(true);
    expect(onSegment(at('bottom'), { x: k, y: n.h }, { x: n.w - k, y: n.h })).toBe(true);
    expect(at('left')).toEqual({ x: 0, y: n.h / 2 });
    expect(at('right')).toEqual({ x: n.w, y: n.h / 2 });
  });

  it('moves a parallelogram\'s side points in onto its slanted sides', () => {
    const n = node('parallelogram');
    const s = slant(n.w, n.h);
    const left = anchorPoint(n, 'left');
    const right = anchorPoint(n, 'right');
    expect(onSegment({ x: left.x - n.x, y: left.y - n.y }, { x: s, y: 0 }, { x: 0, y: n.h })).toBe(true);
    expect(onSegment({ x: right.x - n.x, y: right.y - n.y }, { x: n.w, y: 0 }, { x: n.w - s, y: n.h })).toBe(true);
    expect(anchorInset('parallelogram', 'top', n.w, n.h)).toBe(0);
  });

  it('puts a document\'s bottom point on its wave, and a cylinder\'s on its caps', () => {
    const d = node('document');
    const bottom = anchorPoint(d, 'bottom');
    expect(shapePath('document', d.w, d.h)).toContain(`${d.w / 2},${d.h - wave(d.h)}`);
    expect(bottom.y - d.y).toBe(d.h - wave(d.h));

    const c = node('cylinder');
    expect(anchorPoint(c, 'top').y).toBe(c.y);
    expect(anchorPoint(c, 'bottom').y).toBe(c.y + c.h);
    // The caps are arcs whose apex is the box edge: their centre is one radius in.
    expect(shapePath('cylinder', c.w, c.h)).toContain(`M0,${capRy(c.h)}`);
  });

  it('leaves the original shapes\' connector points at the box midpoints', () => {
    for (const shape of ['circle', 'rect', 'diamond'] as const) {
      const n = node(shape);
      expect(anchorPoint(n, 'left')).toEqual({ x: n.x, y: n.y + n.h / 2 });
      expect(anchorPoint(n, 'bottom')).toEqual({ x: n.x + n.w / 2, y: n.y + n.h });
    }
  });
});
