import {
  ChangeDetectionStrategy, Component, computed, HostListener, inject, signal,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  CanvasEdge, CanvasNode, ModelEditorStore, ModelStatus, StateKind, StateTest,
  NodeShape, NODE_COLORS, safeExternalUrl, GROUP_COLORS, AlignMode, CanvasGroup,
} from '../../state/model-editor.store';
import { TestCaseDialogComponent, TestDraft } from '../test-case-dialog/test-case-dialog';

@Component({
  selector: 'tm-properties-panel',
  imports: [FormsModule, TestCaseDialogComponent],
  templateUrl: './properties-panel.html',
  styleUrl: './properties-panel.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PropertiesPanelComponent {
  readonly store = inject(ModelEditorStore);

  // ── State test cases ──────────────────────────────────────────────────
  readonly testCtx = signal<{ x: number; y: number; nodeId: string; testId: string } | null>(null);

  /** Which test the overlay is editing. */
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

  openTestCtx(e: MouseEvent, nodeId: string, testId: string): void {
    e.preventDefault();
    e.stopPropagation();
    this.testCtx.set({
      x: Math.max(8, Math.min(e.clientX, window.innerWidth  - 200)),
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

  // ── Node bindings ─────────────────────────────────────────────────────
  getNodeLabel(n: CanvasNode): string { return n.label; }
  setNodeLabel(n: CanvasNode, v: string): void { this.store.updateNode(n.id, { label: v }); }

  getNodeKind(n: CanvasNode): StateKind { return n.kind; }
  setNodeKind(n: CanvasNode, v: StateKind): void { this.store.updateNode(n.id, { kind: v }); }

  getNodeDesc(n: CanvasNode): string { return n.description ?? ''; }
  setNodeDesc(n: CanvasNode, v: string): void { this.store.updateNode(n.id, { description: v }); }

  // ── Edge bindings ─────────────────────────────────────────────────────
  getEdgeLabel(e: CanvasEdge): string { return e.label; }
  setEdgeLabel(e: CanvasEdge, v: string): void { this.store.updateEdge(e.id, { label: v }); }

  getEdgeGuard(e: CanvasEdge): string { return e.guard ?? ''; }
  setEdgeGuard(e: CanvasEdge, v: string): void { this.store.updateEdge(e.id, { guard: v || undefined }); }

  getEdgeAction(e: CanvasEdge): string { return e.action ?? ''; }
  setEdgeAction(e: CanvasEdge, v: string): void { this.store.updateEdge(e.id, { action: v || undefined }); }

  nodeName(id: string): string { return this.store.nodeById(id)?.label ?? id.slice(0, 8); }

  // ── Model meta ────────────────────────────────────────────────────────
  statusOptions: ModelStatus[] = ['draft', 'review', 'approved'];

  readonly colors = NODE_COLORS;
  readonly groupColors = GROUP_COLORS;

  readonly alignActions: { mode: AlignMode; label: string; path: string }[] = [
    { mode: 'left',     label: 'Align left',        path: 'M4 3v18M8 7h12M8 17h8' },
    { mode: 'center-h', label: 'Align centre',      path: 'M12 3v18M6 7h12M8 17h8' },
    { mode: 'right',    label: 'Align right',       path: 'M20 3v18M4 7h12M8 17h8' },
    { mode: 'top',      label: 'Align top',         path: 'M3 4h18M7 8v12M17 8v8' },
    { mode: 'middle',   label: 'Align middle',      path: 'M3 12h18M7 6v12M17 8v8' },
    { mode: 'bottom',   label: 'Align bottom',      path: 'M3 20h18M7 4v12M17 8v8' },
    { mode: 'dist-h',   label: 'Distribute across', path: 'M4 3v18M12 3v18M20 3v18' },
    { mode: 'dist-v',   label: 'Distribute down',   path: 'M3 4h18M3 12h18M3 20h18' },
  ];

  selectGroupMembers(g: CanvasGroup): void {
    this.store.selectNodes(this.store.groupMembers(g).map(n => n.id));
  }

  setGroupColor(id: string, color: string): void {
    this.store.checkpoint();
    this.store.updateGroup(id, { color });
  }

  setGroupOpacity(id: string, percent: string): void {
    this.store.updateGroup(id, { opacity: Number(percent) / 100 });
  }
  readonly shapes: { value: NodeShape; label: string }[] = [
    { value: 'circle',    label: 'Circle' },
    { value: 'rect',      label: 'Rectangle' },
    { value: 'diamond',   label: 'Diamond (decision)' },
  ];
}
