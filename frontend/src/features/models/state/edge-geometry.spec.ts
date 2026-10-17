import {
  along, curveThrough, edgeGeometry, facingSide, insertIndex, loopIndices, needsLeader,
  orthogonalPoints, Pt,
} from './edge-geometry';
import { CanvasEdge, CanvasNode } from './model-editor.store';

const node = (id: string, x: number, y: number): CanvasNode => ({
  id, label: id, kind: 'regular', x, y, w: 100, h: 40, shape: 'rect', tests: [],
});
const A = node('a', 0, 0);       // centre (50, 20)
const B = node('b', 400, 0);     // centre (450, 20)
const C = node('c', 400, 300);   // centre (450, 320)
const edge = (partial: Partial<CanvasEdge> = {}): CanvasEdge => ({
  id: 'e', fromId: 'a', toId: 'b', label: 'go', curve: 0, ...partial,
});

/** Points of a path made only of M/L commands. */
function polyline(d: string): Pt[] {
  return d.split(/[ML]/).map(s => s.trim()).filter(Boolean)
    .map(s => { const [x, y] = s.split(' ').map(Number); return { x, y }; });
}

describe('edgeGeometry', () => {
  it('draws a straight transition between the facing outlines, with the bend handle at its middle', () => {
    const g = edgeGeometry(edge(), A, B);
    expect(g.d).toBe('M 102 20 L 398 20');
    expect(g.bend).toEqual({ x: 250, y: 20 });
    expect(g.label).toEqual(g.bend);
  });

  it('puts the bend handle on the curve, so dragging it bends the line through the pointer', () => {
    const curved = edgeGeometry(edge({ curve: 80 }), A, B);
    expect(curved.d.startsWith('M 102 20 Q')).toBe(true);
    // A quadratic's midpoint is half its control offset.
    expect(Math.abs(curved.bend!.y - 20)).toBeCloseTo(40);
    // And curveThrough inverts that.
    expect(curveThrough(curved.src, curved.tgt, curved.bend!)).toBe(80);
  });

  it('passes through waypoints in order with a smooth curve', () => {
    const g = edgeGeometry(edge({ waypoints: [{ x: 200, y: 150 }, { x: 300, y: 150 }] }), A, B);
    expect(g.d).toMatch(/^M [\d.]+ [\d.]+ C .* 200 150 C .* 300 150 C .* [\d.]+ [\d.]+$/);
    expect(g.bend).toBeNull();
  });

  it('routes right-angle transitions with only horizontal and vertical segments', () => {
    const g = edgeGeometry(edge({ toId: 'c', routing: 'orthogonal', fromAnchor: 'right', toAnchor: 'top' }), A, C);
    const pts = polyline(g.d);
    for (let i = 1; i < pts.length; i++) {
      expect(pts[i].x === pts[i - 1].x || pts[i].y === pts[i - 1].y).toBe(true);
    }
    // Leaves to the right, enters from above.
    expect(pts[1].x).toBeGreaterThan(pts[0].x);
    expect(pts[1].y).toBe(pts[0].y);
    expect(pts[pts.length - 2].y).toBeLessThan(pts[pts.length - 1].y);
    expect(pts[pts.length - 1]).toEqual({ x: 450, y: 300 });
  });

  it('picks the facing sides for a right-angle transition without connector dots', () => {
    expect(facingSide(A, { x: 450, y: 20 })).toBe('right');
    expect(facingSide(A, { x: 50, y: 400 })).toBe('bottom');
    const g = edgeGeometry(edge({ routing: 'orthogonal' }), A, B);
    expect(g.src).toEqual({ x: 100, y: 20 });
    expect(g.tgt).toEqual({ x: 400, y: 20 });
  });

  it('draws a self-loop on the side of its connector dot, larger for each further loop', () => {
    const top = edgeGeometry(edge({ toId: 'a' }), A, A, 0);
    const right = edgeGeometry(edge({ toId: 'a', fromAnchor: 'right' }), A, A, 0);
    const second = edgeGeometry(edge({ toId: 'a', fromAnchor: 'right' }), A, A, 1);
    expect(top.labelHome.y).toBeLessThan(A.y);
    expect(right.labelHome.x).toBeGreaterThan(A.x + A.w);
    expect(second.labelHome.x).toBeGreaterThan(right.labelHome.x + 20);
  });

  it('moves the label by its offset and asks for a leader line once it is away', () => {
    const near = edgeGeometry(edge({ labelOffset: { dx: 5, dy: 5 } }), A, B);
    const far = edgeGeometry(edge({ labelOffset: { dx: 0, dy: -60 } }), A, B);
    expect(far.label).toEqual({ x: 250, y: -40 });
    expect(far.labelHome).toEqual({ x: 250, y: 20 });
    expect(needsLeader(near)).toBe(false);
    expect(needsLeader(far)).toBe(true);
  });
});

describe('helpers', () => {
  it('finds the segment a new waypoint belongs to', () => {
    const src = { x: 0, y: 0 }, tgt = { x: 300, y: 0 };
    const wps = [{ x: 100, y: 100 }, { x: 200, y: 100 }];
    expect(insertIndex(src, wps, tgt, { x: 40, y: 50 })).toBe(0);
    expect(insertIndex(src, wps, tgt, { x: 150, y: 105 })).toBe(1);
    expect(insertIndex(src, wps, tgt, { x: 260, y: 40 })).toBe(2);
  });

  it('numbers self-loops per state side and leaves other transitions out', () => {
    const loops = loopIndices([
      edge({ id: '1', toId: 'a' }), edge({ id: '2', toId: 'a' }),
      edge({ id: '3', toId: 'a', fromAnchor: 'left' }), edge({ id: '4' }),
    ]);
    expect([...loops]).toEqual([['1', 0], ['2', 1], ['3', 0]]);
  });

  it('measures along a polyline', () => {
    expect(along([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }], 0.5)).toEqual({ x: 10, y: 0 });
  });

  it('collapses collinear elbows', () => {
    const pts = orthogonalPoints({ x: 0, y: 0 }, 'right', { x: 100, y: 0 }, 'left', []);
    expect(pts).toEqual([{ x: 0, y: 0 }, { x: 100, y: 0 }]);
  });
});
