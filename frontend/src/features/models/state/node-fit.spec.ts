import { fitNodeSize, labelLines, singleLine } from './node-fit';

/** Deterministic 7px per character. */
const m = (t: string) => t.length * 7;
const RECT = { w: 144, h: 48 };
const CIRCLE = { w: 88, h: 88 };
const DIAMOND = { w: 172, h: 104 };

describe('node-fit', () => {
  it('keeps the default size for a short label', () => {
    expect(fitNodeSize('Login', 'rect', RECT, m)).toEqual(RECT);
    expect(fitNodeSize('Start', 'circle', CIRCLE, m)).toEqual(CIRCLE);
    expect(fitNodeSize('Ok?', 'diamond', DIAMOND, m)).toEqual(DIAMOND);
  });

  it('keeps the default size for an empty label', () => {
    expect(fitNodeSize('', 'rect', RECT, m)).toEqual(RECT);
    expect(labelLines('   ', 'rect', m)).toEqual(['']);
  });

  it('wraps a long label and grows a rect in height', () => {
    const label = 'User submits the registration form with valid credentials';
    const lines = labelLines(label, 'rect', m);
    expect(lines.length).toBeGreaterThan(1);
    lines.forEach(l => expect(m(l)).toBeLessThanOrEqual(200));
    const size = fitNodeSize(label, 'rect', RECT, m);
    expect(size.h).toBeGreaterThan(RECT.h);
    expect(size.w).toBeGreaterThanOrEqual(RECT.w);
  });

  it('grows the width for an unbreakable word', () => {
    const size = fitNodeSize('Supercalifragilisticexpialidocious_state', 'rect', RECT, m);
    expect(size.w).toBe(40 * 7 + 24);
  });

  it('keeps a circle round and fits text inside the inscribed area', () => {
    const size = fitNodeSize('Waiting for confirmation', 'circle', CIRCLE, m);
    expect(size.w).toBe(size.h);
    expect(size.w).toBeGreaterThan(CIRCLE.w);
  });

  it('grows a diamond to clear its angled edges', () => {
    const size = fitNodeSize('Is the account verified, active, not suspended and allowed to place orders today?', 'diamond', DIAMOND, m);
    expect(size.w).toBeGreaterThan(DIAMOND.w);
    expect(size.h).toBeGreaterThan(DIAMOND.h);
  });

  it('shrinks back to the default when the label gets short again', () => {
    const long = fitNodeSize('A rather long state name that wraps', 'rect', RECT, m);
    expect(long.h).toBeGreaterThan(RECT.h);
    expect(fitNodeSize('A', 'rect', RECT, m)).toEqual(RECT);
  });

  describe('explicit line breaks', () => {
    it('keeps each line of a name separate', () => {
      expect(labelLines('Login\nForm\nOpen', 'rect', m)).toEqual(['Login', 'Form', 'Open']);
    });

    it('sizes stacked short lines for all of them', () => {
      const size = fitNodeSize('One\nTwo\nThree\nFour', 'rect', RECT, m);
      expect(size.h).toBe(4 * 16 + 20);
      expect(size.w).toBe(RECT.w);
    });

    it('wraps a long line further and counts every resulting line', () => {
      const long = 'User submits the registration form with valid credentials';
      const lines = labelLines(`Start\n${long}`, 'rect', m);
      expect(lines[0]).toBe('Start');
      expect(lines.length).toBeGreaterThan(2);
    });

    it('keeps blank lines in the middle', () => {
      expect(labelLines('A\n\nB', 'rect', m)).toEqual(['A', '', 'B']);
    });

    it('treats CRLF as one break', () => {
      expect(labelLines('A\r\nB', 'rect', m)).toEqual(['A', 'B']);
    });

    it('collapses breaks to spaces for single-line display', () => {
      expect(singleLine('Login\n  Form\n\nOpen')).toBe('Login Form Open');
      expect(singleLine('Plain')).toBe('Plain');
    });
  });
});
