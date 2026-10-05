import {
  ChangeDetectionStrategy, Component, DestroyRef, ElementRef, HostListener,
  computed, effect, inject, signal, viewChild,
} from '@angular/core';
import {
  CanvasNode, ModelEditorStore, StateTest, safeExternalUrl,
} from '../../state/model-editor.store';
import { TestCaseDialogComponent, TestDraft } from '../test-case-dialog/test-case-dialog';

/**
 * The test cases of the model, as the editor's right-hand default tab.
 *
 * Test cases are what the tool is for, so they get a tab of their own rather
 * than a section inside Properties (docs/specification/06-ui.md). With a state
 * selected the tab edits that state's cases; with nothing selected it lists
 * every covered state, so the tab is never empty for no reason.
 */
@Component({
  selector: 'tm-test-cases-panel',
  imports: [TestCaseDialogComponent],
  templateUrl: './test-cases-panel.html',
  styleUrl: './test-cases-panel.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TestCasesPanelComponent {
  readonly store = inject(ModelEditorStore);

  /** The state being edited, when one is selected. */
  readonly node = this.store.selectedNode;

  /** States that carry test cases, for the overview with nothing selected. */
  readonly coveredStates = computed(() =>
    this.store.nodes().filter(n => n.tests.length > 0));

  // ── Revealing the list ────────────────────────────────────────────────────
  private readonly testsSection = viewChild<ElementRef<HTMLElement>>('testsSection');

  /** Brief highlight after the list is scrolled into view. */
  readonly testsFlash = signal(false);

  private flashTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    const destroyRef = inject(DestroyRef);
    destroyRef.onDestroy(() => {
      if (this.flashTimer !== null) clearTimeout(this.flashTimer);
    });

    // Something (the canvas chips) asked for the test cases. The selection that
    // comes with the request has not been rendered yet when the effect runs, so
    // the scroll waits for the next task.
    effect(() => {
      if (this.store.testsFocusTick() === 0) return;
      setTimeout(() => this.revealTests());
    });
  }

  private revealTests(): void {
    const el = this.testsSection()?.nativeElement;
    if (!el) return;
    el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    this.testsFlash.set(true);
    if (this.flashTimer !== null) clearTimeout(this.flashTimer);
    this.flashTimer = setTimeout(() => this.testsFlash.set(false), 1200);
  }

  // ── Context menu ──────────────────────────────────────────────────────────
  readonly testCtx = signal<{ x: number; y: number; nodeId: string; testId: string } | null>(null);

  openTestCtx(e: MouseEvent, nodeId: string, testId: string): void {
    e.preventDefault();
    e.stopPropagation();
    this.testCtx.set({
      x: Math.max(8, Math.min(e.clientX, window.innerWidth - 200)),
      y: Math.max(8, Math.min(e.clientY, window.innerHeight - 130)),
      nodeId, testId,
    });
  }

  closeTestCtx(): void { this.testCtx.set(null); }

  @HostListener('document:mousedown', ['$event'])
  onDocMouseDown(e: MouseEvent): void {
    if (this.testCtx() && !(e.target as Element).closest('.ctx-menu')) this.closeTestCtx();
  }

  @HostListener('document:keydown.escape')
  onEscape(): void { this.closeTestCtx(); }

  editFromCtx(): void {
    const c = this.testCtx();
    this.closeTestCtx();
    if (c) this.openEditor(c.nodeId, c.testId);
  }

  removeFromCtx(): void {
    const c = this.testCtx();
    this.closeTestCtx();
    if (c) this.store.removeTest(c.nodeId, c.testId);
  }

  // ── Overlay editor ────────────────────────────────────────────────────────
  readonly editingIds = signal<{ nodeId: string; testId: string } | null>(null);

  readonly editing = computed(() => {
    const ids = this.editingIds();
    if (!ids) return null;
    const node = this.store.nodeById(ids.nodeId);
    const test = node?.tests.find(t => t.id === ids.testId);
    if (!node || !test) return null;
    return { test, nodeLabel: node.label };
  });

  openEditor(nodeId: string, testId: string): void {
    this.editingIds.set({ nodeId, testId });
  }

  closeEditor(): void { this.editingIds.set(null); }

  saveTest(changes: TestDraft): void {
    const ids = this.editingIds();
    if (ids) this.store.updateTest(ids.nodeId, ids.testId, changes);
    this.closeEditor();
  }

  /** Adds a case to the selected state and opens it for editing straight away. */
  addTest(node: CanvasNode): void {
    const test = this.store.addTest(node.id);
    if (test) this.openEditor(node.id, test.id);
  }

  /** Jump to a state from the overview. */
  focusState(nodeId: string): void { this.store.select(nodeId, 'node'); }

  implHref(t: StateTest):    string | null { return safeExternalUrl(t.implementationUrl); }
  backlogHref(t: StateTest): string | null { return safeExternalUrl(t.backlogUrl); }

  /** One-line Given/When/Then digest. */
  gherkinSummary(test: StateTest): string {
    const part = (kw: string, body: string) => {
      const first = body.split('\n').map(l => l.trim()).find(Boolean);
      return first ? `${kw} ${first}` : '';
    };
    return [
      part('Given', test.given),
      part('When',  test.when),
      part('Then',  test.then),
    ].filter(Boolean).join(' · ') || 'No steps yet';
  }
}
