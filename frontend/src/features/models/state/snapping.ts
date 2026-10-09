/**
 * Where a dragged state lands: on an 8 px grid, or lined up with a nearby
 * state's edge or centre, whichever is close enough. Pure, for tests.
 */
import { Box } from './viewport';
import { GRID } from './shortcuts';

/** A guide line to draw while snapped (canvas coordinates). */
export interface Guide { x1: number; y1: number; x2: number; y2: number }

export interface Snapped { x: number; y: number; guides: Guide[] }

const lines = (start: number, size: number) => [start, start + size / 2, start + size];

/**
 * Snaps `moving` (at its unsnapped position) against `others`.
 * `threshold` is in canvas units: the caller converts from screen pixels.
 */
export function snapPosition(moving: Box, others: Box[], threshold: number): Snapped {
  let x = Math.round(moving.x / GRID) * GRID;
  let y = Math.round(moving.y / GRID) * GRID;
  const guides: Guide[] = [];

  // One axis at a time: the closest alignment within the threshold wins.
  const best = (mine: number[], theirs: (o: Box) => number[]) => {
    let pick: { d: number; at: number; o: Box } | null = null;
    for (const o of others) {
      for (const t of theirs(o)) {
        for (const m of mine) {
          const d = t - m;
          if (Math.abs(d) <= threshold && (!pick || Math.abs(d) < Math.abs(pick.d))) pick = { d, at: t, o };
        }
      }
    }
    return pick;
  };

  const vx = best(lines(moving.x, moving.w), o => lines(o.x, o.w));
  if (vx) {
    x = moving.x + vx.d;
    const top = Math.min(moving.y, vx.o.y), bottom = Math.max(moving.y + moving.h, vx.o.y + vx.o.h);
    guides.push({ x1: vx.at, y1: top - 8, x2: vx.at, y2: bottom + 8 });
  }
  const hy = best(lines(moving.y, moving.h), o => lines(o.y, o.h));
  if (hy) {
    y = moving.y + hy.d;
    const left = Math.min(moving.x, hy.o.x), right = Math.max(moving.x + moving.w, hy.o.x + hy.o.w);
    guides.push({ x1: left - 8, y1: hy.at, x2: right + 8, y2: hy.at });
  }
  return { x, y, guides };
}
