import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  CanvasEdge, CanvasNode, ModelEditorStore, ModelStatus, StateKind,
  NodeShape, NODE_COLORS, GROUP_COLORS, AlignMode, CanvasGroup,
} from '../../state/model-editor.store';

@Component({
  selector: 'tm-properties-panel',
  imports: [FormsModule],
  templateUrl: './properties-panel.html',
  styleUrl: './properties-panel.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PropertiesPanelComponent {
  readonly store = inject(ModelEditorStore);

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
