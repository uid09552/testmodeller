/**
 * Automatic sizing of a state to its label: pure functions, no store access.
 * The label is wrapped at a per-shape maximum text width; the state then grows
 * until the wrapped text fits inside the shape's usable area.
 */
import type { NodeShape } from './model-editor.store';

export const LABEL_LINE_H = 16;
const LABEL_FONT_PX = 13;

/** Widest a wrapped label line may get, per shape. Unbreakable words may exceed it. */
const MAX_TEXT_W: Record<NodeShape, number> = { rect: 200, circle: 110, diamond: 120 };

/** Fraction of width/height that stays inside the outline around the text. */
const INNER_FRAC: Record<NodeShape, number> = { rect: 1, circle: 0.74, diamond: 0.58 };

/** Padding added around the text block of a rectangle. */
const RECT_PAD_W = 24;
const RECT_PAD_H = 20;

export type TextMeasure = (text: string) => number;

let ctx: { font: string; measureText(t: string): { width: number } } | null | undefined;
const cache = new Map<string, number>();

/** Forget measured widths and the font, e.g. once web fonts have loaded. */
export function resetTextMeasure(): void {
  ctx = undefined;
  cache.clear();
}

/** Measures with the canvas font where possible, else a per-character estimate. */
export const measureLabel: TextMeasure = text => {
  if (ctx === undefined) {
    try {
      ctx = typeof OffscreenCanvas === 'undefined' ? null : new OffscreenCanvas(1, 1).getContext('2d');
      if (ctx) {
        const family = getComputedStyle(document.documentElement).getPropertyValue('--font-sans').trim();
        ctx.font = `500 ${LABEL_FONT_PX}px ${family || 'system-ui, sans-serif'}`;
      }
    } catch {
      ctx = null;
    }
  }
  if (!ctx) return text.length * 7;
  let w = cache.get(text);
  if (w === undefined) {
    w = ctx.measureText(text).width;
    cache.set(text, w);
  }
  return w;
};

/** Wraps `label` on spaces so no line exceeds the shape's text width (unless one word does). */
export function labelLines(label: string, shape: NodeShape, measure: TextMeasure = measureLabel): string[] {
  const max = MAX_TEXT_W[shape];
  const words = label.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return [''];
  const lines: string[] = [];
  let line = '';
  for (const word of words) {
    const next = line ? `${line} ${word}` : word;
    if (line && measure(next) > max) {
      lines.push(line);
      line = word;
    } else {
      line = next;
    }
  }
  lines.push(line);
  return lines;
}

/** Smallest size, not below `min`, in which the wrapped label fits the shape. */
export function fitNodeSize(
  label: string, shape: NodeShape, min: { w: number; h: number }, measure: TextMeasure = measureLabel,
): { w: number; h: number } {
  const lines = labelLines(label, shape, measure);
  const textW = Math.max(...lines.map(l => measure(l)));
  const textH = lines.length * LABEL_LINE_H;
  const frac = INNER_FRAC[shape];
  let w: number;
  let h: number;
  if (shape === 'rect') {
    w = textW + RECT_PAD_W;
    h = textH + RECT_PAD_H;
  } else {
    w = textW / frac + 8;
    h = textH / frac + 8;
  }
  w = Math.max(min.w, Math.ceil(w));
  h = Math.max(min.h, Math.ceil(h));
  if (shape === 'circle') w = h = Math.max(w, h);
  return { w, h };
}
