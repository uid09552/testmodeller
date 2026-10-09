import { snapPosition } from './snapping';

describe('snapPosition', () => {
  it('snaps to the 8 px grid when nothing is near', () => {
    const s = snapPosition({ x: 13, y: 21, w: 100, h: 40 }, [], 6);
    expect([s.x, s.y, s.guides]).toEqual([16, 24, []]);
  });

  it('lines up with a neighbour\'s centre and shows a guide', () => {
    const other = { x: 300, y: 100, w: 100, h: 40 };   // centre y = 120
    const s = snapPosition({ x: 13, y: 76, w: 100, h: 90 }, [other], 6); // centre y = 121
    expect(s.y).toBe(75);
    expect(s.guides).toEqual([{ x1: 5, y1: 120, x2: 408, y2: 120 }]);
  });

  it('lines up left edges on the vertical axis', () => {
    const other = { x: 200, y: 400, w: 60, h: 40 };
    const s = snapPosition({ x: 203, y: 37, w: 100, h: 40 }, [other], 6);
    expect(s.x).toBe(200);
    expect(s.guides[0].x1).toBe(200);
  });

  it('ignores neighbours beyond the threshold', () => {
    const s = snapPosition({ x: 13, y: 103, w: 100, h: 40 }, [{ x: 300, y: 200, w: 100, h: 40 }], 6);
    expect(s.guides).toEqual([]);
  });
});
