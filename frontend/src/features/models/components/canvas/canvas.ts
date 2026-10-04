import {
  ChangeDetectionStrategy, ChangeDetectorRef,
  Component, ElementRef, HostListener,
  inject, output, signal, viewChild,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  CanvasEdge, CanvasNode, ModelEditorStore, NODE_H, NODE_W, StateKind,
  NodeShape, SIZE_FOR_SHAPE, SHAPE_FOR_KIND, NODE_COLORS,
  Anchor, ANCHORS, anchorPoint, nearestAnchor,
  AlignMode, CanvasGroup, GROUP_COLORS, ResizeHandle,
} from '../../state/model-editor.store';

/** Shapes offered in the quick bar as drag sources. */
export const PALETTE: { kind: StateKind; label: string }[] = [
  { kind: 'initial',  label: 'Start state (circle)' },
  { kind: 'regular',  label: 'State (rectangle)' },
  { kind: 'decision', label: 'Decision (diamond)' },
  { kind: 'final',    label: 'End state' },
];

interface DragState {
  nodeId: string; startX: number; startY: number; origX: number; origY: number;
  /** Starting positions of every co-selected state, for moving them together. */
  others: { id: string; x: number; y: number }[];
}

/** Rubber-band selection rectangle, in canvas coordinates. */
interface Marquee { x0: number; y0: number; x1: number; y1: number; additive: boolean }

/** Dragging a whole group by its panel. */
interface GroupDrag { groupId: string; startX: number; startY: number }

/** Dragging one of a group's eight resize handles. */
interface GroupResize {
  groupId: string;
  handle: ResizeHandle;
  startX: number;
  startY: number;
  orig: { x: number; y: number; w: number; h: number };
}
interface DrawEdge  { fromId: string; fromAnchor: Anchor; x1: number; y1: number; x2: number; y2: number }

/** Dragging one end of an existing transition onto a different dot. */
interface Reconnect { edgeId: string; end: 'from' | 'to'; x: number; y: number }

export interface ContextMenu {
  /** Viewport coordinates — the menu is position:fixed. */
  screenX: number;
  screenY: number;
  /** World coordinates, for "add state here". */
  canvasX: number;
  canvasY: number;
  type:    'node' | 'edge' | 'canvas' | 'group';
  targetId: string | null;
}

/** Convert mouse event to canvas (SVG user-space) coordinates. */
function toCanvas(e: MouseEvent, svg: SVGSVGElement, zoom: number, panX: number, panY: number) {
  const r = svg.getBoundingClientRect();
  return { x: (e.clientX - r.left - panX) / zoom, y: (e.clientY - r.top - panY) / zoom };
}

/**
 * Where a line from the node's centre toward (tx, ty) leaves the node's
 * outline. Ellipses and diamonds are solved analytically so arrowheads touch
 * the real edge; rectangles use their bounding box, which is the same thing.
 */
function borderPt(n: CanvasNode, tx: number, ty: number) {
  const cx = n.x + n.w / 2, cy = n.y + n.h / 2;
  const dx = tx - cx, dy = ty - cy;
  const len = Math.sqrt(dx * dx + dy * dy);
  if (len < 0.001) return { x: cx, y: cy - n.h / 2 };
  const ndx = dx / len, ndy = dy / len;

  if (n.shape === 'circle') {
    const rx = n.w / 2 + 2, ry = n.h / 2 + 2;
    const k = 1 / Math.sqrt((ndx * ndx) / (rx * rx) + (ndy * ndy) / (ry * ry));
    return { x: cx + ndx * k, y: cy + ndy * k };
  }

  if (n.shape === 'diamond') {
    const a = n.w / 2 + 2, b = n.h / 2 + 2;
    const k = 1 / (Math.abs(ndx) / a + Math.abs(ndy) / b);
    return { x: cx + ndx * k, y: cy + ndy * k };
  }

  const hw = n.w / 2 + 2, hh = n.h / 2 + 2;
  let t = Infinity;
  if (Math.abs(ndx) > 0.001) { const tt = (ndx > 0 ? hw : -hw) / ndx; if (tt > 0) { const y = cy + tt * ndy; if (Math.abs(y - cy) <= hh) t = Math.min(t, tt); } }
  if (Math.abs(ndy) > 0.001) { const tt = (ndy > 0 ? hh : -hh) / ndy; if (tt > 0) { const x = cx + tt * ndx; if (Math.abs(x - cx) <= hw) t = Math.min(t, tt); } }
  if (!isFinite(t)) return { x: cx, y: cy };
  return { x: cx + ndx * t, y: cy + ndy * t };
}

@Component({
  selector: 'tm-canvas',
  imports: [FormsModule],
  templateUrl: './canvas.html',
  styleUrl: './canvas.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CanvasComponent {
  readonly store = inject(ModelEditorStore);
  private readonly cdr = inject(ChangeDetectorRef);

  // ── Viewport ────────────────────────────────────────────────────────────
  readonly panX  = signal(40);
  readonly panY  = signal(40);
  readonly zoom  = signal(1);
  readonly transform = () => `translate(${this.panX()} ${this.panY()}) scale(${this.zoom()})`;

  // ── Interaction state ─────────────────────────────────────────────────
  protected drag     = signal<DragState | null>(null);
  readonly drawing      = signal<DrawEdge | null>(null);
  readonly reconnecting = signal<Reconnect | null>(null);
  readonly marquee      = signal<Marquee | null>(null);
  private  groupDrag    = signal<GroupDrag | null>(null);
  private  groupResize  = signal<GroupResize | null>(null);

  /** Handle positions as fractions of the group rect, plus their cursor. */
  readonly groupHandles: { h: ResizeHandle; fx: number; fy: number; cursor: string }[] = [
    { h: 'nw', fx: 0,   fy: 0,   cursor: 'nwse-resize' },
    { h: 'n',  fx: 0.5, fy: 0,   cursor: 'ns-resize'   },
    { h: 'ne', fx: 1,   fy: 0,   cursor: 'nesw-resize' },
    { h: 'e',  fx: 1,   fy: 0.5, cursor: 'ew-resize'   },
    { h: 'se', fx: 1,   fy: 1,   cursor: 'nwse-resize' },
    { h: 's',  fx: 0.5, fy: 1,   cursor: 'ns-resize'   },
    { h: 'sw', fx: 0,   fy: 1,   cursor: 'nesw-resize' },
    { h: 'w',  fx: 0,   fy: 0.5, cursor: 'ew-resize'   },
  ];

  readonly groupColors = GROUP_COLORS;

  readonly alignActions: { mode: AlignMode; label: string; path: string }[] = [
    { mode: 'left',     label: 'Align left',           path: 'M4 3v18M8 7h12M8 17h8' },
    { mode: 'center-h', label: 'Align centre',         path: 'M12 3v18M6 7h12M8 17h8' },
    { mode: 'right',    label: 'Align right',          path: 'M20 3v18M4 7h12M8 17h8' },
    { mode: 'top',      label: 'Align top',            path: 'M3 4h18M7 8v12M17 8v8' },
    { mode: 'middle',   label: 'Align middle',         path: 'M3 12h18M7 6v12M17 8v8' },
    { mode: 'bottom',   label: 'Align bottom',         path: 'M3 20h18M7 4v12M17 8v8' },
    { mode: 'dist-h',   label: 'Distribute across',    path: 'M4 3v18M12 3v18M20 3v18' },
    { mode: 'dist-v',   label: 'Distribute down',      path: 'M3 4h18M3 12h18M3 20h18' },
  ];
  private panning    = signal<{ sx: number; sy: number; px: number; py: number } | null>(null);
  readonly hoveredNodeId = signal<string | null>(null);

  // ── Quick bar (drag sources) ──────────────────────────────────────────
  readonly palette = PALETTE;
  readonly colors  = NODE_COLORS;
  /** Kind currently being dragged out of the palette. */
  readonly dragKind = signal<StateKind | null>(null);
  readonly dragOver = signal(false);

  onPaletteDragStart(e: DragEvent, kind: StateKind): void {
    this.dragKind.set(kind);
    e.dataTransfer?.setData('text/plain', kind);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy';
  }

  onPaletteDragEnd(): void {
    this.dragKind.set(null);
    this.dragOver.set(false);
  }

  onCanvasDragOver(e: DragEvent): void {
    if (!this.dragKind()) return;
    e.preventDefault();                       // required to allow the drop
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    if (!this.dragOver()) this.dragOver.set(true);
  }

  onCanvasDragLeave(): void { this.dragOver.set(false); }

  /** Drop a palette shape onto the canvas at the cursor. */
  onCanvasDrop(e: DragEvent): void {
    e.preventDefault();
    const kind = this.dragKind()
      ?? (e.dataTransfer?.getData('text/plain') as StateKind | undefined)
      ?? null;
    this.dragOver.set(false);
    this.dragKind.set(null);
    if (!kind) return;

    const shape = SHAPE_FOR_KIND[kind];
    const { w, h } = SIZE_FOR_SHAPE[shape];
    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    const node = this.store.addNode(kind, pt.x - w / 2, pt.y - h / 2);
    this.store.select(node.id, 'node');
    this.cdr.markForCheck();
  }

  /** Apply a border colour to the selected state. */
  applyColor(value: string | null, nodeId?: string): void {
    const id = nodeId ?? (this.store.selected()?.type === 'node' ? this.store.selected()!.id : null);
    if (!id) return;
    this.store.checkpoint();
    this.store.updateNode(id, { color: value });
  }

  /** Change the shape of the selected state. */
  applyShape(shape: NodeShape, nodeId?: string): void {
    const id = nodeId ?? (this.store.selected()?.type === 'node' ? this.store.selected()!.id : null);
    if (!id) return;
    this.store.checkpoint();
    this.store.updateNode(id, { shape });
  }

  /** SVG polygon for a diamond (UML decision) of this size. */
  diamondPoints(n: CanvasNode): string {
    const hw = n.w / 2, hh = n.h / 2;
    return `${hw},0 ${n.w},${hh} ${hw},${n.h} 0,${hh}`;
  }

  /** Only the blue-filled initial state needs light text. */
  labelIsLight(n: CanvasNode): boolean {
    return n.kind === 'initial';
  }

  // ── Context menu ──────────────────────────────────────────────────────
  readonly contextMenu = signal<ContextMenu | null>(null);

  // ── Inline edit ────────────────────────────────────────────────────────
  readonly editNodeId = signal<string | null>(null);
  readonly editValue  = signal('');
  private svgEl = viewChild.required<ElementRef<SVGSVGElement>>('svgCanvas');

  // ── Toolbar actions ───────────────────────────────────────────────────
  addNode(kind: StateKind): void {
    const cx = (400 - this.panX()) / this.zoom();
    const cy = (200 - this.panY()) / this.zoom();
    const node = this.store.addNode(kind, cx - NODE_W / 2, cy - NODE_H / 2);
    this.store.select(node.id, 'node');
  }

  zoomIn():    void { this.zoom.update(z => Math.min(z + 0.15, 3)); }
  zoomOut():   void { this.zoom.update(z => Math.max(z - 0.15, 0.3)); }
  resetView(): void { this.panX.set(40); this.panY.set(40); this.zoom.set(1); }

  /** Delete everything selected, one or many. */
  deleteSelected(): void {
    this.store.deleteSelection();
  }

  alignSelection(mode: AlignMode): void { this.store.alignSelection(mode); }

  /** Mass colour: the whole selection when several, else just the one. */
  applySelectionColor(value: string | null): void {
    if (this.store.hasMultiSelection()) this.store.colorSelection(value);
    else this.applyColor(value);
  }

  // ── Context menu ──────────────────────────────────────────────────────
  openContextMenu(e: MouseEvent, type: 'canvas' | 'node' | 'edge' | 'group', targetId: string | null = null): void {
    e.preventDefault();
    e.stopPropagation();
    const svg = this.svgEl().nativeElement;
    const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
    // The menu is position:fixed, so clamp against the viewport. Menu is
    // ~224px wide; height varies by type (node menu is the tallest at ~310px).
    const menuH = type === 'node' ? 310 : type === 'edge' ? 190 : type === 'group' ? 230 : 200;
    const vx = Math.min(e.clientX, window.innerWidth  - 232);
    const vy = Math.min(e.clientY, window.innerHeight - menuH - 8);
    this.contextMenu.set({
      screenX: Math.max(8, vx),
      screenY: Math.max(8, vy),
      canvasX: pt.x, canvasY: pt.y,
      type, targetId,
    });
    this.cdr.markForCheck();
  }

  closeContextMenu(): void {
    this.contextMenu.set(null);
    this.cdr.markForCheck();
  }

  runMenuAction(action: string): void {
    const m = this.contextMenu();
    this.closeContextMenu();
    if (!m) return;

    switch (action) {
      case 'rename-node': {
        const n = this.store.nodeById(m.targetId!);
        if (n) { this.store.select(n.id, 'node'); this.startEdit(n.id, n.label); }
        break;
      }
      case 'set-initial':
      case 'set-regular':
      case 'set-final': {
        const kind = action.replace('set-', '') as StateKind;
        this.store.checkpoint();
        this.store.updateNode(m.targetId!, { kind });
        break;
      }
      case 'duplicate-node': {
        const n = this.store.nodeById(m.targetId!);
        if (n) {
          const copy = this.store.addNode(n.kind, n.x + 24, n.y + 24);
          this.store.updateNode(copy.id, { label: n.label + ' copy', description: n.description });
          this.store.select(copy.id, 'node');
        }
        break;
      }
      case 'shape-circle':    this.applyShape('circle', m.targetId!); break;
      case 'shape-rect':      this.applyShape('rect', m.targetId!); break;
      case 'shape-diamond':   this.applyShape('diamond', m.targetId!); break;
      case 'add-test': {
        this.store.addTest(m.targetId!);
        this.store.select(m.targetId!, 'node');
        break;
      }
      case 'delete-node':
        this.store.removeNode(m.targetId!);
        break;
      case 'add-transition': {
        const n = this.store.nodeById(m.targetId!);
        if (n) {
          const p = anchorPoint(n, 'right');
          this.drawing.set({
            fromId: n.id, fromAnchor: 'right', x1: p.x, y1: p.y, x2: p.x, y2: p.y,
          });
        }
        break;
      }
      case 'rename-edge': {
        const edge = this.store.edgeById(m.targetId!);
        if (edge) this.store.select(edge.id, 'edge');
        break;
      }
      case 'reverse-edge': {
        const edge = this.store.edgeById(m.targetId!);
        if (edge) this.store.updateEdge(edge.id, { fromId: edge.toId, toId: edge.fromId });
        break;
      }
      case 'delete-edge':
        this.store.removeEdge(m.targetId!);
        break;
      case 'add-state':
        this.store.addNode('regular', m.canvasX - NODE_W / 2, m.canvasY - NODE_H / 2);
        break;
      case 'add-initial':
        this.store.addNode('initial', m.canvasX - NODE_W / 2, m.canvasY - NODE_H / 2);
        break;
      case 'add-final':
        this.store.addNode('final', m.canvasX - NODE_W / 2, m.canvasY - NODE_H / 2);
        break;
      case 'reset-view':
        this.resetView();
        break;
      case 'group-recolor': {
        // Cycle to the next preset so the menu stays a single click.
        const g = this.store.groupById(m.targetId!);
        if (g) {
          const i = GROUP_COLORS.indexOf(g.color);
          this.store.checkpoint();
          this.store.updateGroup(g.id, { color: GROUP_COLORS[(i + 1) % GROUP_COLORS.length] });
        }
        break;
      }
      case 'group-select-members': {
        const g = this.store.groupById(m.targetId!);
        if (g) this.store.selectNodes(this.store.groupMembers(g).map(n => n.id));
        break;
      }
      case 'ungroup':
        this.store.ungroup(m.targetId!);
        break;
      case 'group-fit':
        this.store.fitGroup(m.targetId!);
        break;
      case 'group-delete-all':
        this.store.deleteGroupWithNodes(m.targetId!);
        break;
    }
  }

  // ── Keyboard ──────────────────────────────────────────────────────────
  @HostListener('document:keydown', ['$event'])
  onKeyDown(e: KeyboardEvent): void {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) { e.preventDefault(); this.store.undo(); }
    if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) { e.preventDefault(); this.store.redo(); }
    if ((e.ctrlKey || e.metaKey) && e.key === 'a') { e.preventDefault(); this.store.selectAll(); }
    if ((e.ctrlKey || e.metaKey) && e.key === 'g') { e.preventDefault(); this.groupSelection(); }
    if ((e.key === 'Delete' || e.key === 'Backspace') && this.editNodeId() === null) this.deleteSelected();
    if (e.key === 'Escape') { this.cancelEdit(); this.drawing.set(null); this.reconnecting.set(null); this.marquee.set(null); this.closeContextMenu(); }
  }

  @HostListener('document:mousedown', ['$event'])
  onDocMouseDown(e: MouseEvent): void {
    if (this.contextMenu()) {
      const menu = (e.target as Element).closest('.ctx-menu');
      if (!menu) this.closeContextMenu();
    }
  }

  // ── Scroll to zoom ────────────────────────────────────────────────────
  onWheel(e: WheelEvent): void {
    e.preventDefault();
    const svg  = this.svgEl().nativeElement;
    const rect = svg.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    const newZoom = Math.min(Math.max(this.zoom() * factor, 0.3), 3);
    const ratio = newZoom / this.zoom();
    this.panX.update(px => mx - (mx - px) * ratio);
    this.panY.update(py => my - (my - py) * ratio);
    this.zoom.set(newZoom);
  }

  // ── Canvas background ────────────────────────────────────────────────
  onBgMouseDown(e: MouseEvent): void {
    // Middle-mouse, or Shift/Alt + left, pans. Plain left-drag marquee-selects.
    const wantsPan = e.button === 1 || (e.button === 0 && (e.shiftKey || e.altKey));
    if (wantsPan) {
      e.preventDefault();
      this.panning.set({ sx: e.clientX, sy: e.clientY, px: this.panX(), py: this.panY() });
      return;
    }
    if (e.button !== 0) return;

    const additive = e.ctrlKey || e.metaKey;
    if (!additive) this.store.deselect();
    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    this.marquee.set({ x0: pt.x, y0: pt.y, x1: pt.x, y1: pt.y, additive });
  }

  /** Normalised marquee rect for rendering. */
  marqueeRect(): { x: number; y: number; w: number; h: number } | null {
    const m = this.marquee();
    if (!m) return null;
    return {
      x: Math.min(m.x0, m.x1),
      y: Math.min(m.y0, m.y1),
      w: Math.abs(m.x1 - m.x0),
      h: Math.abs(m.y1 - m.y0),
    };
  }

  // ── Groups ────────────────────────────────────────────────────────────────
  groupSelection(): void {
    const g = this.store.groupSelection();
    if (g) this.revealProperties.emit();
  }

  onGroupMouseDown(e: MouseEvent, group: CanvasGroup): void {
    if (e.button !== 0) return;
    e.stopPropagation();
    if (e.ctrlKey || e.metaKey) this.store.toggleSelect(group.id, 'group');
    else this.store.select(group.id, 'group');
    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    this.groupDrag.set({ groupId: group.id, startX: pt.x, startY: pt.y });
  }

  /** Begin resizing a group from one of its handles. */
  onGroupResizeStart(e: MouseEvent, group: CanvasGroup, handle: ResizeHandle): void {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    this.store.select(group.id, 'group');
    this.store.checkpoint();
    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    this.groupResize.set({
      groupId: group.id, handle,
      startX: pt.x, startY: pt.y,
      orig: { x: group.x, y: group.y, w: group.w, h: group.h },
    });
  }

  fitGroup(id: string): void { this.store.fitGroup(id); }

  /** Remove the grouping; the states stay. */
  deleteGroup(id: string): void { this.store.ungroup(id); }

  onBgDblClick(e: MouseEvent): void {
    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    const node = this.store.addNode('regular', pt.x - NODE_W / 2, pt.y - NODE_H / 2);
    this.store.select(node.id, 'node');
    this.startEdit(node.id, node.label);
  }

  // ── Node interaction ──────────────────────────────────────────────────
  onNodeMouseDown(e: MouseEvent, node: CanvasNode): void {
    if (e.button !== 0) return;
    e.stopPropagation();
    if (this.editNodeId() === node.id) return;

    if (e.ctrlKey || e.metaKey) {
      this.store.toggleSelect(node.id, 'node');
      return;                                  // toggling does not start a drag
    }

    // Dragging a member of a multi-selection moves the whole selection.
    if (!this.store.isSelectedId(node.id)) this.store.select(node.id, 'node');

    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    const others = this.store.selectedNodes()
      .filter(n => n.id !== node.id)
      .map(n => ({ id: n.id, x: n.x, y: n.y }));
    this.drag.set({
      nodeId: node.id, startX: pt.x, startY: pt.y,
      origX: node.x, origY: node.y, others,
    });
  }

  onNodeDblClick(e: MouseEvent, node: CanvasNode): void {
    e.stopPropagation();
    this.startEdit(node.id, node.label);
  }

  // ── Connector (start drawing edge) ────────────────────────────────────
  onConnectorMouseDown(e: MouseEvent, node: CanvasNode, anchor: Anchor): void {
    e.stopPropagation();
    const p = anchorPoint(node, anchor);
    this.drawing.set({
      fromId: node.id, fromAnchor: anchor,
      x1: p.x, y1: p.y, x2: p.x, y2: p.y,
    });
  }

  // ── Edge click ────────────────────────────────────────────────────────
  onEdgeClick(e: MouseEvent, edge: CanvasEdge): void {
    e.stopPropagation();
    this.store.select(edge.id, 'edge');
  }

  /**
   * Double-clicking a transition opens it in the properties panel. Without
   * stopping propagation this would reach the canvas handler, which adds a
   * state at the cursor.
   */
  onEdgeDblClick(e: MouseEvent, edge: CanvasEdge): void {
    e.stopPropagation();
    e.preventDefault();
    this.store.select(edge.id, 'edge');
    this.revealProperties.emit();
  }

  /** Asks the page to un-collapse the properties panel. */
  readonly revealProperties = output<void>();

  /** Public endpoint accessor for the drag handles. */
  edgeEndpoint(e: CanvasEdge, end: 'from' | 'to'): { x: number; y: number } | null {
    const from = this.store.nodeById(e.fromId);
    const to   = this.store.nodeById(e.toId);
    if (!from || !to) return null;
    if (from === to) {
      // Self-loop: handles sit at the two ends of the arc.
      const cx = from.x + from.w / 2;
      return { x: cx + (end === 'from' ? -18 : 18), y: from.y - 2 };
    }
    const { src, tgt } = this.endpoints(e, from, to);
    return end === 'from' ? src : tgt;
  }

  /** Grab one end of a transition to re-attach it elsewhere. */
  onEndpointMouseDown(ev: MouseEvent, edge: CanvasEdge, end: 'from' | 'to'): void {
    ev.stopPropagation();
    ev.preventDefault();
    const p = this.edgeEndpoint(edge, end);
    if (!p) return;
    this.store.select(edge.id, 'edge');
    this.reconnecting.set({ edgeId: edge.id, end, x: p.x, y: p.y });
  }

  /** Ghost line from the fixed end to the cursor while reconnecting. */
  reconnectPath(): string {
    const r = this.reconnecting();
    if (!r) return '';
    const edge = this.store.edgeById(r.edgeId);
    if (!edge) return '';
    const fixed = this.edgeEndpoint(edge, r.end === 'from' ? 'to' : 'from');
    if (!fixed) return '';
    return `M ${fixed.x} ${fixed.y} L ${r.x} ${r.y}`;
  }

  /** Tidy the diagram into layers. */
  alignDiagram(): void { this.store.autoLayout(); }

  // ── Node drop target (when drawing) ──────────────────────────────────
  onNodeMouseUp(e: MouseEvent, node: CanvasNode): void {
    const r = this.reconnecting();
    if (r) {
      const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
      const anchor = nearestAnchor(node, pt.x, pt.y);
      this.store.checkpoint();
      this.store.updateEdge(r.edgeId, r.end === 'from'
        ? { fromId: node.id, fromAnchor: anchor }
        : { toId: node.id,   toAnchor: anchor });
      this.reconnecting.set(null);
      return;
    }

    const d = this.drawing();
    if (!d) return;
    if (d.fromId !== node.id) {
      const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
      this.store.addEdge(d.fromId, node.id, d.fromAnchor, nearestAnchor(node, pt.x, pt.y));
    }
    this.drawing.set(null);
  }

  // ── Global mouse move / up ────────────────────────────────────────────
  @HostListener('document:mousemove', ['$event'])
  onDocMouseMove(e: MouseEvent): void {
    const svg = this.svgEl()?.nativeElement;
    if (!svg) return;

    const pan = this.panning();
    if (pan) {
      this.panX.set(pan.px + (e.clientX - pan.sx));
      this.panY.set(pan.py + (e.clientY - pan.sy));
      this.cdr.markForCheck();
      return;
    }

    const mq = this.marquee();
    if (mq) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      this.marquee.set({ ...mq, x1: pt.x, y1: pt.y });
      this.cdr.markForCheck();
      return;
    }

    const gr = this.groupResize();
    if (gr) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      this.store.resizeGroup(gr.groupId, gr.handle, gr.orig,
        pt.x - gr.startX, pt.y - gr.startY);
      this.cdr.markForCheck();
      return;
    }

    const gd = this.groupDrag();
    if (gd) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      this.store.moveGroup(gd.groupId, pt.x - gd.startX, pt.y - gd.startY);
      this.groupDrag.set({ ...gd, startX: pt.x, startY: pt.y });
      this.cdr.markForCheck();
      return;
    }

    const d = this.drag();
    if (d) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      const dx = pt.x - d.startX, dy = pt.y - d.startY;
      this.store.moveNode(d.nodeId, d.origX + dx, d.origY + dy);
      for (const o of d.others) this.store.moveNode(o.id, o.x + dx, o.y + dy);
      this.cdr.markForCheck();
    }

    const draw = this.drawing();
    if (draw) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      this.drawing.set({ ...draw, x2: pt.x, y2: pt.y });
      this.cdr.markForCheck();
    }

    const recon = this.reconnecting();
    if (recon) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      this.reconnecting.set({ ...recon, x: pt.x, y: pt.y });
      this.cdr.markForCheck();
    }
  }

  @HostListener('document:mouseup', ['$event'])
  onDocMouseUp(e: MouseEvent): void {
    const mq = this.marquee();
    if (mq) {
      const hits = this.store.nodesInRect(mq.x0, mq.y0, mq.x1, mq.y1);
      // A click with no drag just clears; a drag selects what it covered.
      const dragged = Math.abs(mq.x1 - mq.x0) > 3 || Math.abs(mq.y1 - mq.y0) > 3;
      if (dragged) {
        if (mq.additive) for (const id of hits) {
          if (!this.store.isSelectedId(id)) this.store.toggleSelect(id, 'node');
        } else {
          this.store.selectNodes(hits);
        }
      }
      this.marquee.set(null);
    }

    if (this.drag()) this.drag.set(null);
    this.groupDrag.set(null);
    this.groupResize.set(null);
    this.panning.set(null);

    const draw = this.drawing();
    if (draw) {
      // A node's own (mouseup) clears `drawing` before this runs, so if it is
      // still set the transition was released on empty canvas: create the
      // target state there and connect it.
      const svg = this.svgEl()?.nativeElement;
      if (svg && this.isInsideCanvas(e, svg)) {
        const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
        const node = this.store.addNodeWithEdge(
          draw.fromId, pt.x - NODE_W / 2, pt.y - NODE_H / 2, draw.fromAnchor,
        );
        if (node) this.store.select(node.id, 'node');
      }
      this.drawing.set(null);
    }
    // Released off any node: leave the transition where it was.
    if (this.reconnecting()) this.reconnecting.set(null);
    this.cdr.markForCheck();
  }

  /** Guard so releasing over the properties panel does not create a state. */
  private isInsideCanvas(e: MouseEvent, svg: SVGSVGElement): boolean {
    const r = svg.getBoundingClientRect();
    return e.clientX >= r.left && e.clientX <= r.right
        && e.clientY >= r.top  && e.clientY <= r.bottom;
  }

  // ── Inline label edit ─────────────────────────────────────────────────
  startEdit(id: string, label: string): void {
    this.editNodeId.set(id);
    this.editValue.set(label);
    this.cdr.markForCheck();
  }

  finishEdit(): void {
    const id = this.editNodeId();
    if (!id) return;
    const val = this.editValue().trim() || 'State';
    this.store.updateNode(id, { label: val });
    this.editNodeId.set(null);
  }

  cancelEdit(): void {
    this.editNodeId.set(null);
  }

  onEditKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter') { e.preventDefault(); this.finishEdit(); }
    if (e.key === 'Escape') { e.preventDefault(); this.cancelEdit(); }
  }

  // ── Path helpers ──────────────────────────────────────────────────────
  /** Endpoints of an edge: its dots when set, else centre-to-centre geometry. */
  private endpoints(e: CanvasEdge, from: CanvasNode, to: CanvasNode) {
    const src = e.fromAnchor
      ? anchorPoint(from, e.fromAnchor)
      : borderPt(from, to.x + to.w / 2, to.y + to.h / 2);
    const tgt = e.toAnchor
      ? anchorPoint(to, e.toAnchor)
      : borderPt(to, from.x + from.w / 2, from.y + from.h / 2);
    return { src, tgt };
  }

  edgePath(e: CanvasEdge): string {
    const from = this.store.nodeById(e.fromId);
    const to   = this.store.nodeById(e.toId);
    if (!from || !to) return '';

    if (from === to) {
      const cx = from.x + from.w / 2;
      const top = from.y - 2;
      return `M ${cx - 18} ${top} C ${cx - 55} ${top - 72} ${cx + 55} ${top - 72} ${cx + 18} ${top}`;
    }

    const { src, tgt } = this.endpoints(e, from, to);

    if (e.curve === 0) return `M ${src.x} ${src.y} L ${tgt.x} ${tgt.y}`;

    const mx = (src.x + tgt.x) / 2;
    const my = (src.y + tgt.y) / 2;
    const pdx = tgt.y - src.y, pdy = -(tgt.x - src.x);
    const pl = Math.sqrt(pdx * pdx + pdy * pdy) || 1;
    const cpx = mx + (pdx / pl) * e.curve;
    const cpy = my + (pdy / pl) * e.curve;
    return `M ${src.x} ${src.y} Q ${cpx} ${cpy} ${tgt.x} ${tgt.y}`;
  }

  edgeLabelPt(e: CanvasEdge): { x: number; y: number } {
    const from = this.store.nodeById(e.fromId);
    const to   = this.store.nodeById(e.toId);
    if (!from || !to) return { x: 0, y: 0 };
    if (from === to) return { x: from.x + from.w / 2, y: from.y - 52 };

    const { src, tgt } = this.endpoints(e, from, to);
    if (e.curve === 0) return { x: (src.x + tgt.x) / 2, y: (src.y + tgt.y) / 2 };

    const mx = (src.x + tgt.x) / 2, my = (src.y + tgt.y) / 2;
    const pdx = tgt.y - src.y, pdy = -(tgt.x - src.x);
    const pl = Math.sqrt(pdx * pdx + pdy * pdy) || 1;
    return { x: mx + (pdx / pl) * e.curve * 0.5, y: my + (pdy / pl) * e.curve * 0.5 };
  }

  drawingPath(): string {
    const d = this.drawing();
    if (!d) return '';
    return `M ${d.x1} ${d.y1} L ${d.x2} ${d.y2}`;
  }

  readonly anchors = ANCHORS;

  /**
   * Geometry for the inline rename box, centred on the label.
   *
   * It cannot be a fixed rectangle: the shapes differ in size and a circle or
   * diamond tapers, so a box sized for the 144x48 rectangle sat above the
   * label and overlapped the outline.
   */
  editBox(n: CanvasNode): { x: number; y: number; w: number; h: number } {
    const h = 28;
    // Fraction of the width that stays inside the outline at mid-height.
    const frac = n.shape === 'circle'  ? 0.74
               : n.shape === 'diamond' ? 0.58
               : 1;
    const w = n.shape === 'rect' ? n.w - 12 : Math.round(n.w * frac);
    return { x: (n.w - w) / 2, y: n.h / 2 - h / 2, w, h };
  }

  /** Dot position in node-local coordinates (the group is already translated). */
  anchorLocal(n: CanvasNode, a: Anchor): { x: number; y: number } {
    const p = anchorPoint(n, a);
    return { x: p.x - n.x, y: p.y - n.y };
  }

  /**
   * While drawing, the dot that would be used if the pointer were released
   * now — highlighted so the target is obvious before the drop.
   */
  dropAnchor(n: CanvasNode): Anchor | null {
    if (this.hoveredNodeId() !== n.id) return null;
    const d = this.drawing();
    if (d) return d.fromId === n.id ? null : nearestAnchor(n, d.x2, d.y2);
    const r = this.reconnecting();
    if (r) return nearestAnchor(n, r.x, r.y);
    return null;
  }

  isSelected(id: string): boolean { return this.store.isSelectedId(id); }
  isIssue(id: string): boolean { return this.store.issues().some(i => i.elementId === id); }

  nodeFromLabel(id: string): string { return this.store.nodeById(id)?.label ?? id; }
}
