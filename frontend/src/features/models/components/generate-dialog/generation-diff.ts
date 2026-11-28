import { TestCase } from '../../../../core/api/api.types';

/** How a regeneration compares with the model's current generated test cases (06-ui.md). */
export interface GenerationDiff {
  /** Generated now, not before. */
  added: string[];
  /** Generated before and now, with the same steps. */
  unchanged: string[];
  /** Generated before and now, with different steps. */
  changed: string[];
  /** Generated before, not now; saving does not delete them. */
  dropped: string[];
}

function stepsKey(tc: TestCase): string {
  return JSON.stringify(tc.steps.map(s => [s.action, s.expected]));
}

/** Compares by name, which generation derives from the path, then by steps. */
export function diffGeneration(before: TestCase[], after: TestCase[]): GenerationDiff {
  const old = new Map(before.map(tc => [tc.name, stepsKey(tc)]));
  const now = new Set(after.map(tc => tc.name));
  const diff: GenerationDiff = { added: [], unchanged: [], changed: [], dropped: [] };
  for (const tc of after) {
    const prev = old.get(tc.name);
    if (prev === undefined) diff.added.push(tc.name);
    else if (prev === stepsKey(tc)) diff.unchanged.push(tc.name);
    else diff.changed.push(tc.name);
  }
  diff.dropped = before.filter(tc => !now.has(tc.name)).map(tc => tc.name);
  return diff;
}
