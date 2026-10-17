import {
  ChangeDetectionStrategy, Component, computed, ElementRef, input, output, signal, viewChild,
} from '@angular/core';

/** A column: what it shows, and how it sorts (defaults to the text). */
export interface Column<T> {
  key: string;
  label: string;
  text: (row: T) => string;
  sort?: (row: T) => string | number;
}

/**
 * Sortable, filterable table of a threat model's entities. Rows are reached
 * by keyboard: arrows move between rows and Enter opens the row's editor.
 * With `selectable`, a checkbox per row feeds bulk actions.
 */
@Component({
  selector: 'tm-entity-table',
  templateUrl: './entity-table.html',
  styleUrl: './entity-table.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class EntityTableComponent<T extends { id: string }> {
  readonly rows = input.required<T[]>();
  readonly columns = input.required<Column<T>[]>();
  readonly caption = input.required<string>();
  readonly selectable = input(false);
  readonly selectedId = input<string | null>(null);
  /** Text shown when there are no rows at all. */
  readonly emptyText = input('Nothing here yet.');
  readonly open = output<T>();
  readonly checkedChange = output<string[]>();

  readonly filter = signal('');
  readonly sortKey = signal<string | null>(null);
  readonly sortDir = signal<1 | -1>(1);
  readonly checked = signal<ReadonlySet<string>>(new Set());
  private readonly body = viewChild<ElementRef<HTMLElement>>('body');

  readonly visible = computed(() => {
    const q = this.filter().trim().toLowerCase();
    const cols = this.columns();
    let rows = this.rows();
    if (q) rows = rows.filter(r => cols.some(c => c.text(r).toLowerCase().includes(q)));
    const key = this.sortKey();
    const col = cols.find(c => c.key === key);
    if (col) {
      const v = col.sort ?? col.text;
      const dir = this.sortDir();
      rows = [...rows].sort((a, b) => {
        const x = v(a), y = v(b);
        return (x < y ? -1 : x > y ? 1 : 0) * dir;
      });
    }
    return rows;
  });

  readonly allChecked = computed(() =>
    this.visible().length > 0 && this.visible().every(r => this.checked().has(r.id)));

  sortBy(key: string): void {
    if (this.sortKey() === key) this.sortDir.update(d => (d === 1 ? -1 : 1));
    else { this.sortKey.set(key); this.sortDir.set(1); }
  }

  ariaSort(key: string): 'ascending' | 'descending' | 'none' {
    if (this.sortKey() !== key) return 'none';
    return this.sortDir() === 1 ? 'ascending' : 'descending';
  }

  toggle(id: string): void {
    const next = new Set(this.checked());
    if (!next.delete(id)) next.add(id);
    this.setChecked(next);
  }

  toggleAll(): void {
    this.setChecked(this.allChecked() ? new Set() : new Set(this.visible().map(r => r.id)));
  }

  /** Clears the selection, e.g. after a bulk action. */
  clearChecked(): void {
    this.setChecked(new Set());
  }

  private setChecked(s: Set<string>): void {
    this.checked.set(s);
    this.checkedChange.emit([...s]);
  }

  onRowKey(e: KeyboardEvent, row: T, index: number): void {
    if (e.target !== e.currentTarget) return;
    if (e.key === 'Enter' || e.key === 'F2') {
      e.preventDefault();
      this.open.emit(row);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      this.focusRow(index + (e.key === 'ArrowDown' ? 1 : -1));
    } else if (e.key === ' ' && this.selectable()) {
      e.preventDefault();
      this.toggle(row.id);
    }
  }

  private focusRow(i: number): void {
    const rows = this.body()?.nativeElement.querySelectorAll<HTMLElement>('tr');
    if (!rows?.length) return;
    rows[Math.max(0, Math.min(rows.length - 1, i))].focus();
  }
}
