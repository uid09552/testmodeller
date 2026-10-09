import {
  boundsOf, centreOn, fitTo, fromMinimap, isVisible, MAX_FIT_ZOOM, MIN_ZOOM, minimapTransform, visibleBox,
} from './viewport';

const onScreen = (p: { x: number; y: number }, v: { zoom: number; panX: number; panY: number }) =>
  ({ x: p.x * v.zoom + v.panX, y: p.y * v.zoom + v.panY });

describe('viewport', () => {
  it('bounds rectangles', () => {
    expect(boundsOf([])).toBeNull();
    expect(boundsOf([{ x: 0, y: 10, w: 10, h: 10 }, { x: 50, y: 0, w: 10, h: 5 }]))
      .toEqual({ x: 0, y: 0, w: 60, h: 20 });
  });

  it('centres a box at any zoom', () => {
    const box = { x: 100, y: 100, w: 50, h: 50 };
    for (const zoom of [0.5, 1, 2]) {
      const v = centreOn(box, 800, 600, zoom);
      const c = onScreen({ x: 125, y: 125 }, v);
      // Pan is rounded to whole pixels.
      expect(Math.abs(c.x - 400)).toBeLessThanOrEqual(1);
      expect(Math.abs(c.y - 300)).toBeLessThanOrEqual(1);
    }
  });

  it('fits a box with a margin, within the zoom range', () => {
    const v = fitTo({ x: 0, y: 0, w: 1440, h: 400 }, 800, 600);
    const tl = onScreen({ x: 0, y: 0 }, v), br = onScreen({ x: 1440, y: 400 }, v);
    expect(tl.x).toBeGreaterThanOrEqual(39);
    expect(br.x).toBeLessThanOrEqual(761);
    expect(isVisible({ x: 0, y: 0, w: 1440, h: 400 }, v, 800, 600, 30)).toBe(true);
    expect(fitTo({ x: 0, y: 0, w: 10, h: 10 }, 800, 600).zoom).toBe(MAX_FIT_ZOOM);
    expect(fitTo({ x: 0, y: 0, w: 100000, h: 10 }, 800, 600).zoom).toBe(MIN_ZOOM);
  });

  it('knows what is visible', () => {
    const v = { zoom: 2, panX: -100, panY: 0 };
    expect(visibleBox(v, 800, 600)).toEqual({ x: 50, y: -0, w: 400, h: 300 });
    expect(isVisible({ x: 70, y: 20, w: 10, h: 10 }, v, 800, 600)).toBe(true);
    expect(isVisible({ x: 0, y: 20, w: 10, h: 10 }, v, 800, 600)).toBe(false);
  });

  it('maps between the minimap and the canvas', () => {
    const world = { x: -100, y: 0, w: 1000, h: 500 };
    const t = minimapTransform(world, 180, 120);
    const inMini = { x: 500 * t.scale + t.ox, y: 250 * t.scale + t.oy };
    expect(fromMinimap(inMini, t).x).toBeCloseTo(500);
    expect(fromMinimap(inMini, t).y).toBeCloseTo(250);
    // The world fits inside the minimap.
    expect(world.w * t.scale).toBeLessThanOrEqual(180);
    expect(world.h * t.scale).toBeLessThanOrEqual(120);
  });
});
