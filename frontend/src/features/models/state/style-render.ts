/**
 * Turns an `ElementStyle` into what the canvas draws: pure functions, so the
 * mapping can be tested without a DOM.
 *
 * The results are bound as inline styles. Inline styles beat the canvas's CSS
 * defaults, so an absent key leaves the default in place (it maps to null),
 * and the status cues (coverage, path, simulation) win back with
 * `!important` in their own classes.
 */
import { ElementStyle, LineDash, needsLightText, TextSize } from './model-editor.store';

export type InlineStyle = Record<string, string | null>;

/** `stroke-dasharray` for a pattern at a line width, so dashes scale with the line. */
export function dashArray(dash: LineDash | undefined, width: number): string | null {
  if (dash === 'dashed') return `${Math.round(width * 3 + 3)} ${Math.round(width * 2 + 2)}`;
  // Zero-length dashes with round caps draw as dots.
  if (dash === 'dotted') return `0 ${Math.round(width * 2 + 2)}`;
  return null;
}

/** Line colour, width and pattern; `baseWidth` is the element's default width. */
export function linePaint(style: ElementStyle | undefined, baseWidth: number): InlineStyle {
  const width = style?.width ?? baseWidth;
  return {
    stroke: style?.stroke ?? null,
    'stroke-width': style?.width ? `${style.width}px` : null,
    'stroke-dasharray': dashArray(style?.dash, width),
    'stroke-linecap': style?.dash === 'dotted' ? 'round' : null,
  };
}

/** Outline and fill of a shape. */
export function shapePaint(style: ElementStyle | undefined, baseWidth: number): InlineStyle {
  return { ...linePaint(style, baseWidth), fill: style?.fill ?? null };
}

/** Font size in px for a size choice, relative to an element's normal size. */
export function fontPx(size: TextSize | undefined, normal: number): number {
  if (size === 's') return normal - 2;
  if (size === 'l') return normal + 3;
  return normal;
}

/** Text colour, size, weight and slant; `normalPx` is the element's default size. */
export function textPaint(style: ElementStyle | undefined, normalPx: number): InlineStyle {
  return {
    fill: style?.text ?? null,
    'font-size': style?.size ? `${fontPx(style.size, normalPx)}px` : null,
    'font-weight': style?.bold ? '700' : null,
    'font-style': style?.italic ? 'italic' : null,
  };
}

/**
 * Whether a state's label needs light text: never when the user chose a text
 * colour, by contrast on a user fill, and otherwise only on the kind's dark
 * default fill (`darkByDefault`).
 */
export function labelIsLight(style: ElementStyle | undefined, darkByDefault: boolean): boolean {
  if (style?.text) return false;
  if (style?.fill) return needsLightText(style.fill);
  return darkByDefault;
}

/** Marker id for an arrowhead kind in a colour; ids must be valid in `url(#…)`. */
export function arrowMarkerId(kind: 'filled' | 'open' | 'line', colour: string): string {
  return `arrow-${kind}-${colour.replace(/[^0-9a-z]/gi, '')}`;
}
