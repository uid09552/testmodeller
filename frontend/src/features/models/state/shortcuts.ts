/**
 * Every canvas shortcut, in one table: the key handler dispatches on it and
 * the `?` help lists it, so the two cannot drift apart.
 * `focus` shortcuts act only while the canvas has keyboard focus, so they do
 * not take Tab, arrows or Ctrl+F away from the rest of the page.
 */
export type ShortcutId =
  | 'undo' | 'redo' | 'select-all' | 'group' | 'tool-select' | 'tool-pan' | 'delete' | 'escape'
  | 'zoom-fit' | 'zoom-selection' | 'zoom-in' | 'zoom-out' | 'search'
  | 'next-state' | 'prev-state' | 'next-transition' | 'rename'
  | 'move-left' | 'move-right' | 'move-up' | 'move-down' | 'help';

export interface Shortcut {
  id: ShortcutId;
  /** As shown in the help. */
  keys: string;
  label: string;
  focus?: boolean;
  match: (e: KeyboardEvent) => boolean;
}

const ctrl = (e: KeyboardEvent) => e.ctrlKey || e.metaKey;
const plain = (e: KeyboardEvent) => !e.ctrlKey && !e.metaKey && !e.altKey;
const arrow = (key: string) => (e: KeyboardEvent) => plain(e) && e.key === key;

export const SHORTCUTS: Shortcut[] = [
  { id: 'undo', keys: 'Ctrl+Z', label: 'Undo', match: e => ctrl(e) && e.key.toLowerCase() === 'z' && !e.shiftKey },
  { id: 'redo', keys: 'Ctrl+Y / Ctrl+Shift+Z', label: 'Redo',
    match: e => ctrl(e) && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey)) },
  { id: 'select-all', keys: 'Ctrl+A', label: 'Select all states', match: e => ctrl(e) && e.key.toLowerCase() === 'a' },
  { id: 'group', keys: 'Ctrl+G', label: 'Group the selection', match: e => ctrl(e) && e.key.toLowerCase() === 'g' },
  { id: 'search', keys: 'Ctrl+F', label: 'Search states and transitions', focus: true,
    match: e => ctrl(e) && e.key.toLowerCase() === 'f' },
  { id: 'tool-select', keys: 'V', label: 'Select tool', match: e => plain(e) && e.key.toLowerCase() === 'v' },
  { id: 'tool-pan', keys: 'H', label: 'Pan tool', match: e => plain(e) && e.key.toLowerCase() === 'h' },
  { id: 'zoom-fit', keys: 'Shift+1', label: 'Zoom to fit the model',
    match: e => !ctrl(e) && e.shiftKey && (e.key === '!' || e.code === 'Digit1') },
  { id: 'zoom-selection', keys: 'Shift+2', label: 'Zoom to the selection',
    match: e => !ctrl(e) && e.shiftKey && (e.key === '@' || e.key === '"' || e.code === 'Digit2') },
  { id: 'zoom-in', keys: '+', label: 'Zoom in', focus: true, match: e => plain(e) && (e.key === '+' || e.key === '=') },
  { id: 'zoom-out', keys: '-', label: 'Zoom out', focus: true, match: e => plain(e) && e.key === '-' },
  { id: 'next-state', keys: 'Tab', label: 'Next state (reading order)', focus: true,
    match: e => !ctrl(e) && !e.altKey && !e.shiftKey && e.key === 'Tab' },
  { id: 'prev-state', keys: 'Shift+Tab', label: 'Previous state', focus: true,
    match: e => !ctrl(e) && !e.altKey && e.shiftKey && e.key === 'Tab' },
  { id: 'next-transition', keys: 'Ctrl+Tab', label: 'Next transition', focus: true,
    match: e => e.ctrlKey && e.key === 'Tab' },
  { id: 'rename', keys: 'F2', label: 'Rename the selected state or edit a note', focus: true, match: e => e.key === 'F2' },
  { id: 'move-left', keys: 'Arrows', label: 'Move the selection one grid step (Shift: ten)', focus: true,
    match: arrow('ArrowLeft') },
  { id: 'move-right', keys: '', label: '', focus: true, match: arrow('ArrowRight') },
  { id: 'move-up', keys: '', label: '', focus: true, match: arrow('ArrowUp') },
  { id: 'move-down', keys: '', label: '', focus: true, match: arrow('ArrowDown') },
  { id: 'delete', keys: 'Delete / Backspace', label: 'Delete the selection',
    match: e => e.key === 'Delete' || e.key === 'Backspace' },
  { id: 'escape', keys: 'Esc', label: 'Cancel, close menus and search', match: e => e.key === 'Escape' },
  { id: 'help', keys: '?', label: 'Show these shortcuts', match: e => !ctrl(e) && e.key === '?' },
];

/** Shift+arrow uses `plain`, which ignores Shift, so the step is chosen by the handler. */
export const GRID = 8;

/** The rows the help shows: one per action, arrows merged. */
export const HELP_ROWS = SHORTCUTS.filter(s => s.keys);

/** States in reading order: rows (40 px tall) top to bottom, then left to right. */
export function readingOrder<T extends { id: string; x: number; y: number; w: number; h: number }>(nodes: T[]): T[] {
  return [...nodes].sort((a, b) => {
    const ra = Math.round((a.y + a.h / 2) / 40), rb = Math.round((b.y + b.h / 2) / 40);
    return ra - rb || (a.x + a.w / 2) - (b.x + b.w / 2);
  });
}
