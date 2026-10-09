/**
 * Viewport maths for the canvas. The canvas draws
 * `translate(panX panY) scale(zoom)`, so a canvas point p is on screen at
 * `p * zoom + pan`: pan is in screen pixels. Pure, for tests.
 */

export interface Box { x: number; y: number; w: number; h: number }
export interface View { zoom: number; panX: number; panY: number }

export const MIN_ZOOM = 0.3;
export const MAX_ZOOM = 3;
/** Fitting never zooms in further than this, so one small state does not fill the screen. */
export const MAX_FIT_ZOOM = 2;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Bounding box of rectangles, or null for none. */
export function boundsOf(rects: Box[]): Box | null {
  if (!rects.length) return null;
  const x0 = Math.min(...rects.map(r => r.x));
  const y0 = Math.min(...rects.map(r => r.y));
  const x1 = Math.max(...rects.map(r => r.x + r.w));
  const y1 = Math.max(...rects.map(r => r.y + r.h));
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

/** Pan that puts the centre of `box` in the centre of a `w` x `h` view at `zoom`. */
export function centreOn(box: Box, w: number, h: number, zoom: number): View {
  return {
    zoom,
    panX: Math.round(w / 2 - (box.x + box.w / 2) * zoom),
    panY: Math.round(h / 2 - (box.y + box.h / 2) * zoom),
  };
}

/** Zoom and pan so `box` fills the view with `margin` screen pixels around it. */
export function fitTo(box: Box, w: number, h: number, margin = 40): View {
  const zw = (w - 2 * margin) / Math.max(box.w, 1);
  const zh = (h - 2 * margin) / Math.max(box.h, 1);
  const zoom = clamp(Math.min(zw, zh, MAX_FIT_ZOOM), MIN_ZOOM, MAX_ZOOM);
  return centreOn(box, w, h, Math.round(zoom * 100) / 100);
}

/** The canvas area visible in a `w` x `h` view. */
export function visibleBox(view: View, w: number, h: number): Box {
  return { x: -view.panX / view.zoom, y: -view.panY / view.zoom, w: w / view.zoom, h: h / view.zoom };
}

/** True when `box` is inside the view with `margin` screen pixels to spare. */
export function isVisible(box: Box, view: View, w: number, h: number, margin = 24): boolean {
  const left = box.x * view.zoom + view.panX;
  const top = box.y * view.zoom + view.panY;
  return left >= margin && top >= margin
    && left + box.w * view.zoom <= w - margin && top + box.h * view.zoom <= h - margin;
}

/** Scale and offset that fit `world` into a minimap of `mw` x `mh`, centred. */
export function minimapTransform(world: Box, mw: number, mh: number, pad = 6) {
  const scale = Math.min((mw - 2 * pad) / Math.max(world.w, 1), (mh - 2 * pad) / Math.max(world.h, 1));
  return {
    scale,
    ox: (mw - world.w * scale) / 2 - world.x * scale,
    oy: (mh - world.h * scale) / 2 - world.y * scale,
  };
}

/** A point in the minimap back to canvas coordinates. */
export function fromMinimap(p: { x: number; y: number }, t: { scale: number; ox: number; oy: number }) {
  return { x: (p.x - t.ox) / t.scale, y: (p.y - t.oy) / t.scale };
}
