import { ChangeDetectionStrategy, Component, computed, HostListener, inject, signal } from '@angular/core';
import {
  CanvasEdge, CanvasNode, ModelEditorStore, StateKind,
} from '../../state/model-editor.store';
import { singleLine } from '../../state/node-fit';

type Grid = 'states' | 'transitions';
type Row = CanvasNode | CanvasEdge;

/** A column of one of the grids: how to show a cell and, if editable, how to change it. */
interface Column {
  key: string;
  label: string;
  /** `text` and `select` cells can be edited. */
  edit?: 'text' | 'select';
  options?: () => { value: string; label: string }[];
  get: (row: Row) => string;
  /** The raw value an edit starts from. */
  raw?: (row: Row) => string;
  set?: (row: Row, value: string) => void;
}

const KINDS: { value: StateKind; label: string }[] = [
  { value: 'initial', label: 'Initial' }, { value: 'regular', label: 'State' },
  { value: 'decision', label: 'Decision' }, { value: 'final', label: 'Final' },
];

/**
 * The model's states and transitions as two editable grids: the keyboard and
 * screen-reader alternative to the canvas (NFR-004). Edits go through the
 * same store operations and undo history as the canvas.
 *
 * Keyboard (grid pattern, roving tabindex): arrows move between cells, Enter
 * or F2 edits, Enter commits, Esc cancels, Delete deletes the row (pressed
 * twice, to confirm), Tab leaves the grid.
 */
@Component({
  selector: 'tm-model-table',
  templateUrl: './model-table.html',
  styleUrl: './model-table.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ModelTableComponent {
  readonly store = inject(ModelEditorStore);

  readonly grids: { key: Grid; title: string }[] = [
    { key: 'states', title: 'States' }, { key: 'transitions', title: 'Transitions' },
  ];
  readonly states = this.store.nodes;
  readonly transitions = this.store.edges;

  /** The cell that has the grid's single tab stop. */
  readonly active = signal<{ grid: Grid; row: number; col: number }>({ grid: 'states', row: 0, col: 0 });
  /** The cell being edited, with its draft value. */
  readonly editing = signal<{ grid: Grid; row: number; col: number; value: string } | null>(null);
  /** A row waiting for the second Delete. */
  readonly pendingDelete = signal<{ grid: Grid; id: string } | null>(null);

  private name = (id: string) => singleLine(this.store.nodeById(id)?.label ?? '?');
  private stateOptions = () => this.store.nodes().map(n => ({ value: n.id, label: singleLine(n.label) }));
  private commit(fn: () => void): void { this.store.checkpoint(); fn(); }

  readonly stateColumns: Column[] = [
    { key: 'name', label: 'Name', edit: 'text', get: r => singleLine((r as CanvasNode).label),
      raw: r => (r as CanvasNode).label,
      set: (r, v) => this.commit(() => this.store.updateNode(r.id, { label: v.trim() || 'State' })) },
    { key: 'kind', label: 'Kind', edit: 'select', options: () => KINDS,
      get: r => KINDS.find(k => k.value === (r as CanvasNode).kind)?.label ?? '',
      raw: r => (r as CanvasNode).kind,
      set: (r, v) => this.commit(() => this.store.updateNode(r.id, { kind: v as StateKind })) },
    { key: 'tests', label: 'Test cases', get: r => String((r as CanvasNode).tests.length) },
    { key: 'out', label: 'Outgoing', get: r => String(this.store.edges().filter(e => e.fromId === r.id).length) },
  ];

  readonly transitionColumns: Column[] = [
    { key: 'from', label: 'From', edit: 'select', options: this.stateOptions,
      get: r => this.name((r as CanvasEdge).fromId), raw: r => (r as CanvasEdge).fromId,
      set: (r, v) => this.commit(() => this.store.updateEdge(r.id, { fromId: v, fromAnchor: undefined })) },
    { key: 'event', label: 'Event', edit: 'text', get: r => (r as CanvasEdge).label,
      set: (r, v) => this.commit(() => this.store.updateEdge(r.id, { label: v.trim() || 'transition' })) },
    { key: 'guard', label: 'Guard', edit: 'text', get: r => (r as CanvasEdge).guard ?? '',
      set: (r, v) => this.commit(() => this.store.updateEdge(r.id, { guard: v.trim() || undefined })) },
    { key: 'action', label: 'Action', edit: 'text', get: r => (r as CanvasEdge).action ?? '',
      set: (r, v) => this.commit(() => this.store.updateEdge(r.id, { action: v.trim() || undefined })) },
    { key: 'expected', label: 'Expected result', edit: 'text', get: r => (r as CanvasEdge).expected ?? '',
      set: (r, v) => this.commit(() => this.store.updateEdge(r.id, { expected: v.trim() || undefined })) },
    { key: 'to', label: 'To', edit: 'select', options: this.stateOptions,
      get: r => this.name((r as CanvasEdge).toId), raw: r => (r as CanvasEdge).toId,
      set: (r, v) => this.commit(() => this.store.updateEdge(r.id, { toId: v, toAnchor: undefined })) },
  ];

  /** Validation issues per element, as text. */
  readonly issues = computed(() => {
    const out = new Map<string, string>();
    for (const i of this.store.issues()) {
      if (!i.elementId) continue;
      out.set(i.elementId, [out.get(i.elementId), `${i.severity}: ${i.message}`].filter(Boolean).join(' '));
    }
    return out;
  });

  rows(grid: Grid): Row[] { return grid === 'states' ? this.states() : this.transitions(); }
  columns(grid: Grid): Column[] { return grid === 'states' ? this.stateColumns : this.transitionColumns; }

  isActive(grid: Grid, row: number, col: number): boolean {
    const a = this.active();
    return a.grid === grid && a.row === row && a.col === col;
  }
  isEditing(grid: Grid, row: number, col: number): boolean {
    const e = this.editing();
    return !!e && e.grid === grid && e.row === row && e.col === col;
  }

  focusCell(grid: Grid, row: number, col: number): void {
    this.active.set({ grid, row, col });
    setTimeout(() => document.getElementById(this.cellId(grid, row, col))?.focus());
  }

  cellId(grid: Grid, row: number, col: number): string { return `mt-${grid}-${row}-${col}`; }

  startEdit(grid: Grid, row: number, col: number): void {
    const c = this.columns(grid)[col];
    const r = this.rows(grid)[row];
    if (!c?.edit || !r) return;
    this.editing.set({ grid, row, col, value: (c.raw ?? c.get)(r) });
    setTimeout(() => document.getElementById(this.cellId(grid, row, col) + '-edit')?.focus());
  }

  draft(value: string): void { this.editing.update(e => e && { ...e, value }); }

  finishEdit(save: boolean): void {
    const e = this.editing();
    if (!e) return;
    this.editing.set(null);
    if (save) {
      const r = this.rows(e.grid)[e.row];
      const c = this.columns(e.grid)[e.col];
      if (r && c.set && (c.raw ?? c.get)(r) !== e.value) c.set(r, e.value);
    }
    this.focusCell(e.grid, e.row, e.col);
  }

  onEditorKey(e: KeyboardEvent): void {
    e.stopPropagation();
    if (e.key === 'Enter') { e.preventDefault(); this.finishEdit(true); }
    if (e.key === 'Escape') { e.preventDefault(); this.finishEdit(false); }
  }

  onCellKey(e: KeyboardEvent, grid: Grid, row: number, col: number): void {
    const rows = this.rows(grid).length, cols = this.columns(grid).length;
    const move = (r: number, c: number) => {
      e.preventDefault();
      this.pendingDelete.set(null);
      this.focusCell(grid, Math.max(0, Math.min(rows - 1, r)), Math.max(0, Math.min(cols - 1, c)));
    };
    switch (e.key) {
      case 'ArrowDown': move(row + 1, col); break;
      case 'ArrowUp': move(row - 1, col); break;
      case 'ArrowRight': move(row, col + 1); break;
      case 'ArrowLeft': move(row, col - 1); break;
      case 'Home': move(row, 0); break;
      case 'End': move(row, cols - 1); break;
      case 'Enter': case 'F2': e.preventDefault(); this.startEdit(grid, row, col); break;
      case 'Delete': e.preventDefault(); this.requestDelete(grid, row); break;
      case 'Escape': this.pendingDelete.set(null); break;
    }
  }

  requestDelete(grid: Grid, row: number): void {
    const r = this.rows(grid)[row];
    if (!r) return;
    const p = this.pendingDelete();
    if (p && p.grid === grid && p.id === r.id) {
      this.pendingDelete.set(null);
      if (grid === 'states') this.store.removeNode(r.id); else this.store.removeEdge(r.id);
      const left = this.rows(grid).length;
      if (left) this.focusCell(grid, Math.min(row, left - 1), this.active().col);
    } else {
      this.pendingDelete.set({ grid, id: r.id });
    }
  }

  addState(): void {
    const n = this.store.nodes();
    const y = n.length ? Math.max(...n.map(x => x.y + x.h)) + 60 : 80;
    this.store.addNode(n.length ? 'regular' : 'initial', 80, y);
    this.focusCell('states', this.store.nodes().length - 1, 0);
  }

  addTransition(): void {
    const n = this.store.nodes();
    if (!n.length) return;
    this.store.addEdge(n[0].id, (n[1] ?? n[0]).id);
    this.focusCell('transitions', this.store.edges().length - 1, 0);
  }

  /** Undo and redo, as on the canvas (which is not on screen in table view). */
  @HostListener('document:keydown', ['$event'])
  onKey(e: KeyboardEvent): void {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k === 'z' && !e.shiftKey) { e.preventDefault(); this.store.undo(); }
    if (k === 'y' || (k === 'z' && e.shiftKey)) { e.preventDefault(); this.store.redo(); }
  }
}
