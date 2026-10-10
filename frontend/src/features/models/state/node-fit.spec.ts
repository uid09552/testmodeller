import { fitNodeSize, labelFont, labelLineH, labelLines, singleLine, TextMeasure } from './node-fit';
import { SIZE_FOR_SHAPE } from './model-editor.store';

/** Deterministic 7px per character. */
const m = (t: string) => t.length * 7;
const RECT = { w: 144, h: 48 };
const CIRCLE = { w: 88, h: 88 };
const DIAMOND = { w: 172, h: 104 };

describe('node-fit', () => {
  it('keeps the default size for a short label', () => {
    expect(fitNodeSize('Login', 'rect', RECT, undefined, m)).toEqual(RECT);
    expect(fitNodeSize('Start', 'circle', CIRCLE, undefined, m)).toEqual(CIRCLE);
    expect(fitNodeSize('Ok?', 'diamond', DIAMOND, undefined, m)).toEqual(DIAMOND);
  });

  it('keeps the default size for an empty label', () => {
    expect(fitNodeSize('', 'rect', RECT, undefined, m)).toEqual(RECT);
    expect(labelLines('   ', 'rect', undefined, m)).toEqual(['']);
  });

  it('wraps a long label and grows a rect in height', () => {
    const label = 'User submits the registration form with valid credentials';
    const lines = labelLines(label, 'rect', undefined, m);
    expect(lines.length).toBeGreaterThan(1);
    lines.forEach(l => expect(m(l)).toBeLessThanOrEqual(200));
    const size = fitNodeSize(label, 'rect', RECT, undefined, m);
    expect(size.h).toBeGreaterThan(RECT.h);
    expect(size.w).toBeGreaterThanOrEqual(RECT.w);
  });

  it('grows the width for an unbreakable word', () => {
    const size = fitNodeSize('Supercalifragilisticexpialidocious_state', 'rect', RECT, undefined, m);
    expect(size.w).toBe(40 * 7 + 24);
  });

  it('keeps a circle round and fits text inside the inscribed area', () => {
    const size = fitNodeSize('Waiting for confirmation', 'circle', CIRCLE, undefined, m);
    expect(size.w).toBe(size.h);
    expect(size.w).toBeGreaterThan(CIRCLE.w);
  });

  it('grows a diamond to clear its angled edges', () => {
    const size = fitNodeSize('Is the account verified, active, not suspended and allowed to place orders today?', 'diamond', DIAMOND, undefined, m);
    expect(size.w).toBeGreaterThan(DIAMOND.w);
    expect(size.h).toBeGreaterThan(DIAMOND.h);
  });

  it('shrinks back to the default when the label gets short again', () => {
    const long = fitNodeSize('A rather long state name that wraps', 'rect', RECT, undefined, m);
    expect(long.h).toBeGreaterThan(RECT.h);
    expect(fitNodeSize('A', 'rect', RECT, undefined, m)).toEqual(RECT);
  });

  describe('explicit line breaks', () => {
    it('keeps each line of a name separate', () => {
      expect(labelLines('Login\nForm\nOpen', 'rect', undefined, m)).toEqual(['Login', 'Form', 'Open']);
    });

    it('sizes stacked short lines for all of them', () => {
      const size = fitNodeSize('One\nTwo\nThree\nFour', 'rect', RECT, undefined, m);
      expect(size.h).toBe(4 * 16 + 20);
      expect(size.w).toBe(RECT.w);
    });

    it('wraps a long line further and counts every resulting line', () => {
      const long = 'User submits the registration form with valid credentials';
      const lines = labelLines(`Start\n${long}`, 'rect', undefined, m);
      expect(lines[0]).toBe('Start');
      expect(lines.length).toBeGreaterThan(2);
    });

    it('keeps blank lines in the middle', () => {
      expect(labelLines('A\n\nB', 'rect', undefined, m)).toEqual(['A', '', 'B']);
    });

    it('treats CRLF as one break', () => {
      expect(labelLines('A\r\nB', 'rect', undefined, m)).toEqual(['A', 'B']);
    });

    it('collapses breaks to spaces for single-line display', () => {
      expect(singleLine('Login\n  Form\n\nOpen')).toBe('Login Form Open');
      expect(singleLine('Plain')).toBe('Plain');
    });
  });
});

describe('node-fit with a text style', () => {
  /** Width scales with the font size; bold is 10% wider. */
  const fm: TextMeasure = (t, f) => t.length * 7 * (f.px / 13) * (f.weight > 500 ? 1.1 : 1);
  const label = 'Waiting for a customer\nto pay';

  it('measures large and bold labels wider, so the state grows', () => {
    const normal = fitNodeSize(label, 'rect', RECT, undefined, fm);
    const large = fitNodeSize(label, 'rect', RECT, { size: 'l' }, fm);
    const bold = fitNodeSize(label, 'rect', RECT, { bold: true }, fm);
    expect(large.w).toBeGreaterThan(normal.w);
    expect(large.h).toBeGreaterThan(normal.h);
    expect(bold.w).toBeGreaterThan(normal.w);
  });

  it('never goes below the shape\'s default size when the label shrinks back', () => {
    expect(fitNodeSize('Ok', 'rect', RECT, { size: 's' }, fm)).toEqual(RECT);
    expect(fitNodeSize('Ok', 'rect', RECT, undefined, fm)).toEqual(RECT);
  });

  it('scales the line height with the font size', () => {
    expect([labelLineH({ size: 's' }), labelLineH(), labelLineH({ size: 'l' })]).toEqual([14, 16, 20]);
    expect(labelFont({ size: 'l', bold: true, italic: true })).toEqual({ px: 16, weight: 700, italic: true });
  });

  it('wraps sooner at a larger size', () => {
    const long = 'Order is waiting for payment';
    expect(labelLines(long, 'rect', { size: 'l' }, fm).length)
      .toBeGreaterThan(labelLines(long, 'rect', undefined, fm).length);
  });

  it('fits the new shapes around their text, with room for their outline', () => {
    for (const shape of ['hexagon', 'parallelogram', 'cylinder', 'document'] as const) {
      const min = SIZE_FOR_SHAPE[shape];
      const size = fitNodeSize('A considerably longer state name', shape, min, undefined, fm);
      expect(size.w).toBeGreaterThan(min.w);
      expect(size.h).toBeGreaterThanOrEqual(min.h);
    }
  });
});
