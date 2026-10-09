import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  ModelEditorStore, StateTest, TestCategory, gherkinClause, safeExternalUrl,
} from '../../state/model-editor.store';
import { TestCaseDialogComponent } from '../test-case-dialog/test-case-dialog';

type CategoryFilter = 'all' | TestCategory;

@Component({
  selector: 'tm-bottom-panel',
  imports: [FormsModule, TestCaseDialogComponent],
  templateUrl: './bottom-panel.html',
  styleUrl: './bottom-panel.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BottomPanelComponent {
  readonly store = inject(ModelEditorStore);
  readonly activeTab = this.store.bottomTab;

  // ── Test case list ─────────────────────────────────────────────────────────
  readonly categoryFilters: CategoryFilter[] = ['all', 'unit', 'integration', 'feature'];
  readonly categoryFilter = signal<CategoryFilter>('all');

  readonly visibleTests = computed(() => {
    const f = this.categoryFilter();
    const rows = this.store.allTests();
    return f === 'all' ? rows : rows.filter(r => r.test.category === f);
  });

  readonly statesWithTests = computed(() =>
    this.store.nodes().filter(n => n.tests.length > 0).length);

  implHref(t: StateTest):    string | null { return safeExternalUrl(t.implementationUrl); }
  backlogHref(t: StateTest): string | null { return safeExternalUrl(t.backlogUrl); }

  /** One-line Given/When/Then digest for the table. */
  gherkinSummary(test: StateTest): string {
    const part = (kw: string, body: string) => {
      const first = body.split('\n').map(l => l.trim()).find(Boolean);
      return first ? `${kw} ${first}` : '';
    };
    return [
      part('Given', test.given),
      part('When',  test.when),
      part('Then',  test.then),
    ].filter(Boolean).join('  ·  ') || 'No steps yet';
  }

  focusState(nodeId: string): void { this.store.select(nodeId, 'node'); }

  // ── Overlay editor ─────────────────────────────────────────────────────────
  readonly editingIds = signal<{ nodeId: string; testId: string } | null>(null);

  readonly editing = computed(() => {
    const ids = this.editingIds();
    if (!ids) return null;
    const node = this.store.nodeById(ids.nodeId);
    const test = node?.tests.find(t => t.id === ids.testId);
    if (!node || !test) return null;
    return { test, nodeId: node.id, nodeLabel: node.label };
  });

  editTest(nodeId: string, testId: string): void {
    this.editingIds.set({ nodeId, testId });
  }

  closeEditor(): void { this.editingIds.set(null); }

  saveTest(changes: Partial<Omit<StateTest, 'id'>>): void {
    const ids = this.editingIds();
    if (ids) this.store.updateTest(ids.nodeId, ids.testId, changes);
    this.closeEditor();
  }

  // ── Export ─────────────────────────────────────────────────────────────────
  exportGherkin(): void {
    const rows = this.visibleTests();
    if (!rows.length) return;
    const body = rows
      .map(({ test, ref, nodeLabel }) => {
        const tags = `  @${test.category} @${test.polarity} @id:${ref}`;
        const steps = [
          ...gherkinClause('Given', test.given),
          ...gherkinClause('When',  test.when),
          ...gherkinClause('Then',  test.then),
        ];
        return [tags, `  # state: ${nodeLabel}`, `  Scenario: ${test.name}`, ...steps].join('\n');
      })
      .join('\n\n');

    const text = `Feature: ${this.store.name()}\n\n${body}\n`;
    const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${this.store.name().replace(/\s+/g, '-').toLowerCase()}.feature`;
    a.click();
    URL.revokeObjectURL(url);
  }
}
