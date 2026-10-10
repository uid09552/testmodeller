/**
 * Automatic sizing of a state to its label: pure functions, no store access.
 * The label is wrapped at a per-shape maximum text width; the state then grows
 * until the wrapped text fits inside the shape's usable area.
 */
import type { ElementStyle, NodeShape } from './model-editor.store';
import { fontPx } from './style-render';

/** Line height and font size of a normal label. */
export const LABEL_LINE_H = 16;
export const LABEL_FONT_PX = 13;

/** The font a label is drawn in, from its style. */
export interface LabelFont { px: number; weight: number; italic: boolean }

export function labelFont(style?: ElementStyle): LabelFont {
  return { px: fontPx(style?.size, LABEL_FONT_PX), weight: style?.bold ? 700 : 500, italic: !!style?.italic };
}

/** Line height of a label in its style; scales with the font size. */
export function labelLineH(style?: ElementStyle): number {
  return Math.round(fontPx(style?.size, LABEL_FONT_PX) * LABEL_LINE_H / LABEL_FONT_PX);
}

/** Widest a wrapped label line may get, per shape. Unbreakable words may exceed it. */
const MAX_TEXT_W: Record<NodeShape, number> = {
  rect: 200, circle: 110, diamond: 120, hexagon: 180, parallelogram: 180, cylinder: 160, document: 190,
};

/**
 * How a shape's size follows its text: tapering shapes keep the text within a
 * fraction of their width and height (`frac`); the others add padding
 * around it (`pad`), which covers a hexagon's points, a parallelogram's
 * slant, a cylinder's caps and a document's wave.
 */
type Fit = { frac: number } | { padW: number; padH: number };
const FIT: Record<NodeShape, Fit> = {
  rect:          { padW: 24, padH: 20 },
  circle:        { frac: 0.74 },
  diamond:       { frac: 0.58 },
  hexagon:       { padW: 52, padH: 20 },
  parallelogram: { padW: 56, padH: 20 },
  cylinder:      { padW: 24, padH: 40 },
  document:      { padW: 24, padH: 30 },
};

/** Fraction of a shape's width that the label may use at mid-height. */
export function innerWidthFrac(shape: NodeShape): number {
  const fit = FIT[shape];
  return 'frac' in fit ? fit.frac : 1;
}

export type TextMeasure = (text: string, font: LabelFont) => number;

let ctx: { font: string; measureText(t: string): { width: number } } | null | undefined;
let family = '';
const cache = new Map<string, number>();

/** Forget measured widths and the font, e.g. once web fonts have loaded. */
export function resetTextMeasure(): void {
  ctx = undefined;
  cache.clear();
}

/** Measures with the canvas font where possible, else a per-character estimate. */
export const measureLabel: TextMeasure = (text, font) => {
  if (ctx === undefined) {
    try {
      ctx = typeof OffscreenCanvas === 'undefined' ? null : new OffscreenCanvas(1, 1).getContext('2d');
      if (ctx) {
        family = getComputedStyle(document.documentElement).getPropertyValue('--font-sans').trim()
          || 'system-ui, sans-serif';
      }
    } catch {
      ctx = null;
    }
  }
  // Without a canvas: about 7 px per character at 13 px, bold a little wider.
  if (!ctx) return text.length * 7 * (font.px / LABEL_FONT_PX) * (font.weight > 500 ? 1.08 : 1);
  const css = `${font.italic ? 'italic ' : ''}${font.weight} ${font.px}px ${family}`;
  const key = `${css}|${text}`;
  let w = cache.get(key);
  if (w === undefined) {
    ctx.font = css;
    w = ctx.measureText(text).width;
    cache.set(key, w);
  }
  return w;
};

/**
 * Splits `label` on its explicit line breaks, then wraps each line on spaces so
 * none exceeds the shape's text width (unless one word does). Blank lines stay.
 */
export function labelLines(
  label: string, shape: NodeShape, style?: ElementStyle, measure: TextMeasure = measureLabel,
): string[] {
  return wrapText(label, MAX_TEXT_W[shape], style, measure);
}

/** `text` split on its line breaks and wrapped on spaces at `max` px. */
export function wrapText(
  text: string, max: number, style?: ElementStyle, measure: TextMeasure = measureLabel,
): string[] {
  const font = labelFont(style);
  const lines: string[] = [];
  for (const raw of text.split(/\r?\n/)) {
    const words = raw.trim().split(/\s+/).filter(Boolean);
    if (words.length === 0) { lines.push(''); continue; }
    let line = '';
    for (const word of words) {
      const next = line ? `${line} ${word}` : word;
      if (line && measure(next, font) > max) {
        lines.push(line);
        line = word;
      } else {
        line = next;
      }
    }
    lines.push(line);
  }
  return lines;
}

/** A name on one line, for places that cannot show breaks (messages, tree). */
export function singleLine(label: string): string {
  return label.split(/\s*\r?\n\s*/).filter(Boolean).join(' ');
}

/** Smallest size, not below `min`, in which the wrapped label fits the shape. */
export function fitNodeSize(
  label: string, shape: NodeShape, min: { w: number; h: number }, style?: ElementStyle,
  measure: TextMeasure = measureLabel,
): { w: number; h: number } {
  const font = labelFont(style);
  const lines = labelLines(label, shape, style, measure);
  const textW = Math.max(...lines.map(l => measure(l, font)));
  const textH = lines.length * labelLineH(style);
  const fit = FIT[shape];
  let w: number;
  let h: number;
  if ('padW' in fit) {
    w = textW + fit.padW;
    h = textH + fit.padH;
  } else {
    w = textW / fit.frac + 8;
    h = textH / fit.frac + 8;
  }
  w = Math.max(min.w, Math.ceil(w));
  h = Math.max(min.h, Math.ceil(h));
  if (shape === 'circle') w = h = Math.max(w, h);
  return { w, h };
}
