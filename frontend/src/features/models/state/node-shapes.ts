/**
 * Outlines of the state shapes, in a state's local coordinates (0,0 is its
 * top-left corner): pure functions shared by the canvas, the shape pickers
 * and the tests.
 *
 * Every shape touches its bounding box at the middle of each side, so the
 * four connector points sit at the box's midpoints, except where an outline
 * is inset there: the slanted sides of a parallelogram and the wavy bottom
 * of a document. `anchorInset` gives those offsets.
 */
import type { Anchor, NodeShape } from './model-editor.store';

/** Shapes in the order the pickers offer them. */
export const ALL_SHAPES: { value: NodeShape; label: string }[] = [
  { value: 'circle',        label: 'Circle' },
  { value: 'rect',          label: 'Rectangle' },
  { value: 'diamond',       label: 'Diamond (decision)' },
  { value: 'hexagon',       label: 'Hexagon' },
  { value: 'parallelogram', label: 'Parallelogram' },
  { value: 'cylinder',      label: 'Cylinder' },
  { value: 'document',      label: 'Document' },
];

/** Horizontal inset of a hexagon's points and a parallelogram's slant. */
export const slant = (w: number, h: number): number => Math.min(h * 0.5, w * 0.2);
/** Height of a cylinder's elliptical cap (half of it shows on each end). */
export const capRy = (h: number): number => Math.min(10, h * 0.15);
/** Amplitude of a document's wavy bottom. */
export const wave = (h: number): number => Math.min(8, h * 0.14);

const r = (v: number) => Math.round(v * 100) / 100;

/** SVG path data of a shape's outline in a w x h box. */
export function shapePath(shape: NodeShape, w: number, h: number): string {
  switch (shape) {
    case 'circle': {
      const rx = w / 2, ry = h / 2;
      return `M0,${r(ry)} A${r(rx)},${r(ry)} 0 1 1 ${r(w)},${r(ry)} A${r(rx)},${r(ry)} 0 1 1 0,${r(ry)} Z`;
    }
    case 'diamond':
      return `M${r(w / 2)},0 L${r(w)},${r(h / 2)} L${r(w / 2)},${r(h)} L0,${r(h / 2)} Z`;
    case 'hexagon': {
      const k = slant(w, h);
      return `M${r(k)},0 L${r(w - k)},0 L${r(w)},${r(h / 2)} L${r(w - k)},${r(h)} L${r(k)},${r(h)} L0,${r(h / 2)} Z`;
    }
    case 'parallelogram': {
      const s = slant(w, h);
      return `M${r(s)},0 L${r(w)},0 L${r(w - s)},${r(h)} L0,${r(h)} Z`;
    }
    case 'cylinder': {
      const ry = capRy(h), rx = w / 2;
      // Body with the back of the top cap, then the visible front of the top cap.
      return `M0,${r(ry)} A${r(rx)},${r(ry)} 0 0 1 ${r(w)},${r(ry)} L${r(w)},${r(h - ry)}`
        + ` A${r(rx)},${r(ry)} 0 0 1 0,${r(h - ry)} Z`
        + ` M0,${r(ry)} A${r(rx)},${r(ry)} 0 0 0 ${r(w)},${r(ry)}`;
    }
    case 'document': {
      const a = wave(h);
      return `M0,0 L${r(w)},0 L${r(w)},${r(h - a)}`
        + ` Q${r(w * 0.75)},${r(h - 3 * a)} ${r(w / 2)},${r(h - a)}`
        + ` Q${r(w * 0.25)},${r(h + a)} 0,${r(h - a)} Z`;
    }
    case 'rect':
    default: {
      const k = Math.min(8, w / 2, h / 2);
      return `M${k},0 L${r(w - k)},0 Q${r(w)},0 ${r(w)},${k} L${r(w)},${r(h - k)} Q${r(w)},${r(h)} ${r(w - k)},${r(h)}`
        + ` L${k},${r(h)} Q0,${r(h)} 0,${r(h - k)} L0,${k} Q0,0 ${k},0 Z`;
    }
  }
}

/** How far a connector point sits inside the bounding box on its side. */
export function anchorInset(shape: NodeShape, a: Anchor, w: number, h: number): number {
  if (shape === 'parallelogram' && (a === 'left' || a === 'right')) return slant(w, h) / 2;
  if (shape === 'document' && a === 'bottom') return wave(h);
  return 0;
}

/** Icon proportions per shape, within a 24 x 24 box. */
const ICON_SIZE: Record<NodeShape, { w: number; h: number }> = {
  circle: { w: 16, h: 16 }, rect: { w: 20, h: 11 }, diamond: { w: 18, h: 14 },
  hexagon: { w: 20, h: 12 }, parallelogram: { w: 20, h: 11 }, cylinder: { w: 14, h: 18 },
  document: { w: 18, h: 14 },
};

/** A shape's outline for a 24 x 24 icon, centred: path and its offset. */
export function shapeIcon(shape: NodeShape): { d: string; transform: string } {
  const { w, h } = ICON_SIZE[shape];
  return { d: shapePath(shape, w, h), transform: `translate(${(24 - w) / 2} ${(24 - h) / 2})` };
}
