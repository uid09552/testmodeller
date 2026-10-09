import {
  ChangeDetectionStrategy, Component, computed, inject, output, signal,
} from '@angular/core';
import { AutofocusDirective } from '../../../../shared/components/autofocus';
import { ModelEditorStore, SearchHit } from '../../state/model-editor.store';

/**
 * Find states and transitions by name, event or guard (Ctrl+F on the
 * canvas). Each hit the user moves to is passed to the canvas to select and
 * bring into view.
 */
@Component({
  selector: 'tm-canvas-search',
  imports: [AutofocusDirective],
  templateUrl: './canvas-search.html',
  styleUrl: './canvas-search.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CanvasSearchComponent {
  private readonly store = inject(ModelEditorStore);
  readonly show = output<SearchHit>();
  readonly closed = output<void>();

  readonly query = signal('');
  readonly index = signal(0);
  readonly hits = computed(() => this.store.searchElements(this.query()));
  readonly status = computed(() => {
    if (!this.query().trim()) return '';
    const n = this.hits().length;
    return n ? `${this.index() + 1} of ${n}` : 'No matches';
  });

  setQuery(q: string): void {
    this.query.set(q);
    this.index.set(0);
    const first = this.hits()[0];
    if (first) this.show.emit(first);
  }

  step(dir: 1 | -1): void {
    const n = this.hits().length;
    if (!n) return;
    this.index.update(i => (i + dir + n) % n);
    this.show.emit(this.hits()[this.index()]);
  }

  onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter') { e.preventDefault(); this.step(e.shiftKey ? -1 : 1); }
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); this.closed.emit(); }
  }
}
