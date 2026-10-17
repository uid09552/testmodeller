import { component, flow, zone } from '../testing/fixtures';
import {
  emptyLayout, exitPoint, NODE_H, NODE_W, parseLayout, resolveLayout, zoneAt,
} from './diagram-layout';

describe('diagram-layout', () => {
  it('ignores what it does not understand', () => {
    expect(parseLayout(null)).toEqual(emptyLayout());
    expect(parseLayout({ v: 2, nodes: { a: { x: 1, y: 1, w: 1, h: 1 } } })).toEqual(emptyLayout());
    const l = parseLayout({
      v: 1,
      nodes: { a: { x: 1, y: 2, w: 3, h: 4, shape: 'hexagon', style: { fill: '#ff0000', dash: 'wavy', bold: true } }, b: { x: 'no' } },
      edges: { g: { fromAnchor: 'bottom', toAnchor: 'sideways', routing: 'orthogonal', style: { arrow: 'open', stroke: 'url(evil)' } } },
    });
    expect(Object.keys(l.nodes)).toEqual(['a']);
    expect(l.nodes['a'].shape).toBe('hexagon');
    expect(l.nodes['a'].style).toEqual({ fill: '#ff0000', bold: true });
    expect(l.edges['g']).toEqual({ fromAnchor: 'bottom', routing: 'orthogonal', style: { arrow: 'open' } });
  });

  it('reads the bends of layouts saved before flows had dots', () => {
    expect(parseLayout({ v: 1, curves: { f: 40 } }).edges['f']).toEqual({ curve: 40 });
  });

  it('keeps known positions, drops deleted entities and places new ones without overlap', () => {
    const stored = { ...emptyLayout(), nodes: { a: { x: 40, y: 40, w: NODE_W, h: NODE_H }, gone: { x: 0, y: 0, w: 10, h: 10 } } };
    const l = resolveLayout(stored, [component('a', 'A'), component('b', 'B')], [], []);
    expect(l.nodes['a']).toEqual(stored.nodes.a);
    expect(l.nodes['gone']).toBeUndefined();
    const b = l.nodes['b'];
    const overlap = b.x < 40 + NODE_W && 40 < b.x + NODE_W && b.y < 40 + NODE_H && 40 < b.y + NODE_H;
    expect(overlap).toBe(false);
  });

  it('draws a zone around its members and keeps them inside', () => {
    const l = resolveLayout(emptyLayout(),
      [component('a', 'A', { zoneId: 'z' }), component('b', 'B', { zoneId: 'z' })], [zone('z', 'Internal', 3)], []);
    const z = l.zones['z'];
    for (const id of ['a', 'b']) {
      const n = l.nodes[id];
      expect(n.x >= z.x && n.y >= z.y && n.x + n.w <= z.x + z.w && n.y + n.h <= z.y + z.h).toBe(true);
    }
    expect(zoneAt({ x: l.nodes['a'].x + 10, y: l.nodes['a'].y + 10 }, l)).toBe('z');
  });

  it('fans out parallel flows between the same pair', () => {
    const l = resolveLayout(emptyLayout(), [component('a', 'A'), component('b', 'B')], [],
      [flow('f1', 'a', 'b'), flow('f2', 'a', 'b'), flow('f3', 'b', 'a')]);
    expect(new Set([l.edges['f1'].curve, l.edges['f2'].curve, l.edges['f3'].curve]).size).toBe(3);
  });

  it('keeps a stored bend, dots and style of a flow', () => {
    const stored = { ...emptyLayout(), edges: { f1: { curve: 12, fromAnchor: 'top' as const, style: { stroke: '#00f' } } } };
    const l = resolveLayout(stored, [component('a', 'A'), component('b', 'B')], [], [flow('f1', 'a', 'b')]);
    expect(l.edges['f1']).toEqual({ curve: 12, fromAnchor: 'top', style: { stroke: '#00f' } });
  });

  it('finds where a line leaves a region', () => {
    const r = { x: 0, y: 0, w: 100, h: 100 };
    expect(exitPoint({ x: 50, y: 50 }, { x: 250, y: 50 }, r)).toEqual({ x: 100, y: 50 });
    expect(exitPoint({ x: 50, y: 50 }, { x: 60, y: 60 }, r)).toBeNull();
    expect(exitPoint({ x: 500, y: 50 }, { x: 50, y: 50 }, r)).toBeNull();
  });
});
