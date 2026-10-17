import { arrowMarkerId, dashArray, fontPx, labelIsLight, linePaint, shapePaint, textPaint } from './style-render';

describe('style rendering', () => {
  it('maps an absent style to no inline values, so the CSS defaults apply', () => {
    expect(Object.values(shapePaint(undefined, 1.5)).every(v => v === null)).toBe(true);
    expect(Object.values(textPaint(undefined, 13)).every(v => v === null)).toBe(true);
  });

  it('scales dashes and dots with the line width', () => {
    expect(dashArray('dashed', 1)).toBe('6 4');
    expect(dashArray('dashed', 3)).toBe('12 8');
    expect(dashArray('dotted', 2)).toBe('0 6');
    expect(dashArray(undefined, 2)).toBeNull();
  });

  it('draws dotted lines with round caps, and uses the user width', () => {
    expect(linePaint({ dash: 'dotted', width: 4, stroke: '#ff0000' }, 1.5)).toEqual({
      stroke: '#ff0000', 'stroke-width': '4px', 'stroke-dasharray': '0 10', 'stroke-linecap': 'round',
    });
  });

  it('includes the fill for shapes', () => {
    expect(shapePaint({ fill: '#123456' }, 1.5)['fill']).toBe('#123456');
  });

  it('sets text colour, size, weight and slant', () => {
    expect(textPaint({ text: '#00aa00', size: 'l', bold: true, italic: true }, 13)).toEqual({
      fill: '#00aa00', 'font-size': '16px', 'font-weight': '700', 'font-style': 'italic',
    });
    expect(fontPx('s', 11)).toBe(9);
  });

  it('switches to light text on a dark fill, unless a text colour is set', () => {
    expect(labelIsLight({ fill: '#1e293b' }, false)).toBe(true);
    expect(labelIsLight({ fill: '#fef3c7' }, true)).toBe(false);
    expect(labelIsLight({ fill: '#1e293b', text: '#ff0000' }, false)).toBe(false);
    expect(labelIsLight(undefined, true)).toBe(true);
  });

  it('builds marker ids that are safe in url()', () => {
    expect(arrowMarkerId('open', '#Ef4444')).toBe('arrow-open-Ef4444');
  });
});
