import {
  ChangeDetectionStrategy, ChangeDetectorRef, computed, afterNextRender, DestroyRef,
  Component, ElementRef, HostListener,
  effect, inject, input, output, signal, untracked, viewChild,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import {
  CanvasEdge, CanvasNode, ModelEditorStore, NODE_H, NODE_W, StateKind,
  NodeShape, SIZE_FOR_SHAPE, SHAPE_FOR_KIND, NODE_COLORS,
  Anchor, ANCHORS, anchorPoint, nearestAnchor,
  AlignMode, CanvasGroup, GROUP_COLORS, ResizeHandle, CoverageView, coverageLabel, collapsedNode,
  COLLAPSED_W, COLLAPSED_H, SearchHit, AnnotationKind, ANNOTATION_SIZE,
} from '../../state/model-editor.store';
import { innerWidthFrac, LABEL_FONT_PX, labelLineH, labelLines } from '../../state/node-fit';
import { InlineStyle, labelIsLight, shapePaint, textPaint } from '../../state/style-render';
import { ALL_SHAPES, shapeIcon, shapePath } from '../../state/node-shapes';

/** Shapes with no SVG primitive of their own. */
const PATH_SHAPES = new Set<NodeShape>(['hexagon', 'parallelogram', 'cylinder', 'document']);
import {
  curveThrough, EdgeGeometry, edgeGeometry, insertIndex, loopIndices,
} from '../../state/edge-geometry';
import { AnnotationGrab, CanvasAnnotationsComponent } from '../canvas-annotations/canvas-annotations';
import { CanvasEdgesComponent, EdgeGrab } from '../canvas-edges/canvas-edges';
import { CanvasSearchComponent } from '../canvas-search/canvas-search';
import { CanvasMinimapComponent } from '../canvas-minimap/canvas-minimap';
import { CanvasHelpComponent } from '../canvas-help/canvas-help';
import { readJson, writeJson } from '../../../../core/persistence/local-store';
import { boundsOf, Box, centreOn, fitTo, isVisible, View } from '../../state/viewport';
import { GRID, HELP_ROWS, readingOrder, ShortcutId, SHORTCUTS } from '../../state/shortcuts';

/** Shortcuts that change the model, off in present mode. */
const EDIT_SHORTCUTS = new Set<ShortcutId>([
  'undo', 'redo', 'group', 'delete', 'rename', 'move-left', 'move-right', 'move-up', 'move-down',
]);
import { Guide, snapPosition } from '../../state/snapping';

/** Shapes offered in the quick bar as drag sources. */
export const PALETTE: { kind: StateKind; label: string }[] = [
  { kind: 'initial',  label: 'Start state (circle)' },
  { kind: 'regular',  label: 'State (rectangle)' },
  { kind: 'decision', label: 'Decision (diamond)' },
  { kind: 'final',    label: 'End state' },
];

/** Palette tools for elements that are not part of the model. */
export const ANNOTATION_TOOLS: { kind: AnnotationKind; label: string }[] = [
  { kind: 'note', label: 'Note' },
  { kind: 'text', label: 'Text' },
];

/**
 * What a left-drag on the canvas does.
 *
 * `select` is the editing default: drag the background to marquee-select, drag
 * a state to move it. `pan` is the hand tool: a left-drag moves the view
 * wherever it starts, which is what you want while reading a diagram you do
 * not mean to change.
 */
export type CanvasTool = 'select' | 'pan';

/** Test-chip geometry; the template and `testsTransform` must agree on it. */
const TEST_CHIP_W = 31;
const TEST_CHIP_STEP = 34;

interface DragState {
  /** The grabbed state or annotation (`isNote`). */
  nodeId: string; isNote?: boolean; startX: number; startY: number; origX: number; origY: number;
  /** Starting positions of every co-selected state, for moving them together. */
  others: { id: string; x: number; y: number }[];
  /** Starting positions of every co-selected annotation. */
  notes: { id: string; x: number; y: number }[];
  /** Set on the first move, which records the drag's one undo step. */
  moved?: boolean;
}

/** Dragging one of an annotation's resize handles. */
interface NoteResize {
  id: string; handle: ResizeHandle; startX: number; startY: number;
  orig: { x: number; y: number; w: number; h: number };
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


@Component({
  selector: 'tm-canvas',
  imports: [
    FormsModule, CanvasEdgesComponent, CanvasAnnotationsComponent, CanvasSearchComponent,
    CanvasMinimapComponent, CanvasHelpComponent,
  ],
  templateUrl: './canvas.html',
  styleUrl: './canvas.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '[class.readonly]': 'readonly()' },
})
export class CanvasComponent {
  readonly store = inject(ModelEditorStore);
  private readonly cdr = inject(ChangeDetectorRef);

  constructor() {
    const destroyRef = inject(DestroyRef);
    afterNextRender(() => {
      const svg = this.svgEl().nativeElement;
      const measure = () => {
        const r = svg.getBoundingClientRect();
        this.canvasSize.set({ w: r.width, h: r.height });
      };
      measure();
      if (typeof ResizeObserver !== 'undefined') {
        const ro = new ResizeObserver(measure);
        ro.observe(svg);
        destroyRef.onDestroy(() => ro.disconnect());
      }
    });
    // Label widths depend on the web font, which may arrive after first paint.
    void document.fonts?.ready.then(() => this.store.refitNodes());
    // Bring an element into view when something else (e.g. a validation row) asks.
    effect(() => {
      const req = this.store.revealRequest();
      if (req) untracked(() => this.revealElement(req.id));
    });
  }

  // ── Viewport ────────────────────────────────────────────────────────────
  readonly panX  = signal(40);
  readonly panY  = signal(40);
  readonly zoom  = signal(1);
  readonly transform = () => `translate(${this.panX()} ${this.panY()}) scale(${this.zoom()})`;

  // ── Interaction state ─────────────────────────────────────────────────
  protected drag     = signal<DragState | null>(null);
  private readonly noteResize = signal<NoteResize | null>(null);
  private readonly annotationLayer = viewChild(CanvasAnnotationsComponent);
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
  readonly annotationTools = ANNOTATION_TOOLS;
  readonly colors  = NODE_COLORS;
  readonly shapes  = ALL_SHAPES;
  readonly shapeIcon = shapeIcon;
  /** Kind currently being dragged out of the palette: a state or an annotation. */
  readonly dragKind = signal<StateKind | AnnotationKind | null>(null);
  readonly dragOver = signal(false);

  onPaletteDragStart(e: DragEvent, kind: StateKind | AnnotationKind): void {
    this.dragKind.set(kind);
    e.dataTransfer?.setData('text/plain', kind);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy';
  }

  onPaletteDragEnd(): void {
    this.dragKind.set(null);
    this.dragOver.set(false);
  }

  onCanvasDragOver(e: DragEvent): void {
    if (this.readonly()) return;
    if (!this.dragKind()) return;
    e.preventDefault();                       // required to allow the drop
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    if (!this.dragOver()) this.dragOver.set(true);
  }

  onCanvasDragLeave(): void { this.dragOver.set(false); }

  /** Drop a palette shape onto the canvas at the cursor. */
  onCanvasDrop(e: DragEvent): void {
    if (this.readonly()) return;
    e.preventDefault();
    const kind = this.dragKind()
      ?? (e.dataTransfer?.getData('text/plain') as StateKind | AnnotationKind | undefined)
      ?? null;
    this.dragOver.set(false);
    this.dragKind.set(null);
    if (!kind) return;

    if (kind === 'note' || kind === 'text') {
      const { w, h } = ANNOTATION_SIZE[kind];
      const at = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
      const a = this.store.addAnnotation(kind, at.x - w / 2, at.y - h / 2);
      this.store.select(a.id, 'annotation');
      this.cdr.markForCheck();
      this.editAnnotation(a.id);
      return;
    }

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
    this.store.setStyle([id], 'stroke', value);
  }

  /** Change the shape of the selected state. */
  applyShape(shape: NodeShape, nodeId?: string): void {
    const id = nodeId ?? (this.store.selected()?.type === 'node' ? this.store.selected()!.id : null);
    if (!id) return;
    this.store.checkpoint();
    this.store.updateNode(id, { shape });
  }

  /** SVG polygon for a diamond (UML decision) of this size. */
  /** Shapes drawn from `node-shapes.ts` paths rather than SVG primitives. */
  isPathShape(n: CanvasNode): boolean { return PATH_SHAPES.has(n.shape); }

  shapePathOf(n: CanvasNode): string { return shapePath(n.shape, n.w, n.h); }

  diamondPoints(n: CanvasNode): string {
    const hw = n.w / 2, hh = n.h / 2;
    return `${hw},0 ${n.w},${hh} ${hw},${n.h} 0,${hh}`;
  }


  // ── Coverage overlay ──────────────────────────────────────────────────
  /** e.g. "3/5 states, 4/7 transitions (as of last save)". */
  readonly coverageSummary = computed(() => {
    const parts: string[] = [];
    // "passing" only once the model has results; before that it would always be 0.
    const passing = (c: { passing?: number }) =>
      this.store.hasResults() && c.passing !== undefined ? ` (${c.passing} passing)` : '';
    if (this.store.showStateGaps()) {
      const sc = this.store.stateCoverage();
      parts.push(`${coverageLabel(sc)} states${passing(sc)}`);
    }
    if (this.store.showTransitionGaps()) {
      const tc = this.store.transitionCoverage();
      parts.push(!tc ? 'transitions unavailable until saved'
        : `${coverageLabel(tc)} transitions${passing(tc)}${this.store.transitionCoverageStale() ? ' (as of last save)' : ''}`);
    }
    return parts.join(', ');
  });
  /** The dimension the toggle turns back on. */
  private lastCoverageView: Exclude<CoverageView, 'off'> = 'both';
  /** Where the last press on a state began, so a drag is not taken for a click. */
  private nodeDownAt: { x: number; y: number } | null = null;

  toggleCoverage(): void {
    const cur = this.store.coverageView();
    this.store.coverageView.set(cur === 'off' ? this.lastCoverageView : 'off');
  }

  setCoverageView(view: string): void {
    if (view !== 'states' && view !== 'transitions' && view !== 'both') return;
    this.lastCoverageView = view;
    this.store.coverageView.set(view);
  }

  /** Clicking a highlighted state opens its test cases, ready to add one. */
  onNodeClick(e: MouseEvent, node: CanvasNode): void {
    const down = this.nodeDownAt;
    this.nodeDownAt = null;
    if (down && Math.hypot(e.clientX - down.x, e.clientY - down.y) > 4) return;
    if (this.store.isUncoveredState(node.id)) this.revealTestsFor(node);
  }

  nodeAriaLabel(node: CanvasNode): string {
    let label = node.kind + ' state: ' + node.label;
    if (this.store.isUncoveredState(node.id)) label += ', not covered by any test case';
    const steps = this.store.pathSteps(node.id);
    if (steps?.length) label += ', path step ' + steps.join(', ');
    return label;
  }

  /** "1, 3" for an element on the highlighted path. */
  stepText(id: string): string { return (this.store.pathSteps(id) ?? []).join(', '); }

  edgeAriaLabel(edge: CanvasEdge): string {
    let label = 'Transition: ' + edge.label;
    if (this.store.isUncoveredTransition(edge.id)) label += ', not covered by any test case';
    const steps = this.store.pathSteps(edge.id);
    if (steps?.length) label += ', path step ' + steps.join(', ');
    return label;
  }

  /** The label wrapped as the sizing logic did, so it fits the state. */
  labelLines(n: CanvasNode): string[] { return labelLines(n.label, n.shape, n.style); }

  lineHeightOf(n: CanvasNode): number { return labelLineH(n.style); }

  /** Baseline of the first line, so the block is centred vertically. */
  labelY(n: CanvasNode, lineCount: number): number {
    const lh = labelLineH(n.style);
    return n.h / 2 + lh * 0.3 - ((lineCount - 1) * lh) / 2;
  }

  /** Light text on a dark fill: the initial state's blue, or a dark user fill. */
  labelIsLight(n: CanvasNode): boolean {
    return labelIsLight(n.style, n.kind === 'initial');
  }

  /** The user's outline and fill for a state's body. */
  nodePaint(n: CanvasNode): InlineStyle {
    return shapePaint(n.style, n.style?.stroke ? 2.5 : 1.5);
  }

  /** The user's text style for a state's label. */
  labelPaint(n: CanvasNode): InlineStyle { return textPaint(n.style, LABEL_FONT_PX); }

  // ── Context menu ──────────────────────────────────────────────────────
  readonly contextMenu = signal<ContextMenu | null>(null);

  // ── Inline edit ────────────────────────────────────────────────────────
  readonly editNodeId = signal<string | null>(null);
  readonly editValue  = signal('');
  /** The name before editing began, restored if the edit is cancelled. */
  private editOriginal = '';
  private svgEl = viewChild.required<ElementRef<SVGSVGElement>>('svgCanvas');

  // ── Toolbar actions ───────────────────────────────────────────────────
  addNode(kind: StateKind): void {
    const cx = (400 - this.panX()) / this.zoom();
    const cy = (200 - this.panY()) / this.zoom();
    const node = this.store.addNode(kind, cx - NODE_W / 2, cy - NODE_H / 2);
    this.store.select(node.id, 'node');
  }

  // ── Tool ──────────────────────────────────────────────────────────────────
  readonly tool = signal<CanvasTool>('select');

  setTool(tool: CanvasTool): void {
    this.tool.set(tool);
    // Anything half-started with the other tool would be stranded.
    this.drawing.set(null);
    this.reconnecting.set(null);
    this.marquee.set(null);
  }

  /** True while the hand tool should take a left-drag, whatever it lands on. */
  private handTool(e: MouseEvent): boolean {
    return this.tool() === 'pan' && e.button === 0;
  }

  /** Starts a pan from this event. Returns true when it took the event. */
  private startPan(e: MouseEvent): boolean {
    e.preventDefault();
    e.stopPropagation();
    this.panning.set({ sx: e.clientX, sy: e.clientY, px: this.panX(), py: this.panY() });
    return true;
  }

  /** Cursor for the canvas: the tool, unless something is already happening. */
  canvasCursor(): string | null {
    if (this.drawing() || this.reconnecting()) return 'crosshair';
    if (this.panning()) return 'grabbing';
    return this.tool() === 'pan' ? 'grab' : null;
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
    // Every entry changes the model; present mode has none.
    if (this.readonly()) return;
    const svg = this.svgEl().nativeElement;
    const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
    // The menu is position:fixed, so clamp against the viewport. Menu is
    // ~224px wide; height varies by type (node menu is the tallest at ~310px).
    const menuH = type === 'node' ? 310 : type === 'edge' ? 330 : type === 'group' ? 230 : 200;
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
    if (action.startsWith('shape-')) {
      this.applyShape(action.slice('shape-'.length) as NodeShape, m.targetId!);
      return;
    }

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
      case 'add-bend-point':
        this.addBendPoint(m.targetId!, { x: m.canvasX, y: m.canvasY });
        break;
      case 'straighten':
        this.store.straighten(m.targetId!);
        break;
      case 'routing-orthogonal':
        this.store.setRouting(m.targetId!, 'orthogonal');
        break;
      case 'routing-curved':
        this.store.setRouting(m.targetId!, 'curved');
        break;
      case 'reset-label':
        this.store.resetLabelOffset(m.targetId!);
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
      case 'group-collapse':
        this.store.collapseGroup(m.targetId!);
        break;
      case 'group-expand':
        this.store.expandGroup(m.targetId!);
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
  // ── Keyboard (see state/shortcuts.ts) ──────────────────────────────────
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  readonly helpOpen = signal(false);
  /** Present mode: the model can be looked at, navigated and highlighted, not changed. */
  readonly readonly = input(false);
  readonly searchOpen = signal(false);

  // ── Snapping ──────────────────────────────────────────────────────────
  /** Remembered per browser. Alt suspends it while dragging. */
  readonly snapOn = signal(readJson<boolean>('editor-snap') ?? true);
  /** Guide lines while a dragged state is lined up with a neighbour. */
  readonly guides = signal<Guide[]>([]);

  toggleSnap(): void {
    this.snapOn.update(v => !v);
    writeJson('editor-snap', this.snapOn());
  }

  // ── Minimap ───────────────────────────────────────────────────────────
  /** Remembered per browser; a convenience, not part of the model. */
  readonly minimapOn = signal(readJson<boolean>('editor-minimap') ?? true);
  /** Size of the canvas on screen, kept current by a ResizeObserver. */
  readonly canvasSize = signal({ w: 0, h: 0 });
  readonly currentView = computed(() => ({ zoom: this.zoom(), panX: this.panX(), panY: this.panY() }));
  readonly minimapBoxes = computed<Box[]>(() => [
    ...this.visibleNodes(),
    ...this.store.groups().filter(g => g.collapsed).map(collapsedNode),
  ]);

  toggleMinimap(): void {
    this.minimapOn.update(v => !v);
    writeJson('editor-minimap', this.minimapOn());
  }

  /** Centres the view on a canvas point (from the minimap). */
  panToPoint(p: { x: number; y: number }): void {
    const { w, h } = this.canvasSize();
    if (!w) return;
    this.applyView(centreOn({ x: p.x, y: p.y, w: 0, h: 0 }, w, h, this.zoom()));
  }

  openSearch(): void { this.searchOpen.set(true); }

  closeSearch(): void {
    this.searchOpen.set(false);
    this.host.nativeElement.querySelector<HTMLElement>('.canvas-wrap')?.focus();
  }

  /** A search hit: select it and bring it into view. */
  showHit(hit: SearchHit): void {
    this.store.select(hit.id, hit.type);
    this.revealElement(hit.id);
  }
  readonly helpRows = HELP_ROWS;
  readonly collapsedW = COLLAPSED_W;
  readonly collapsedH = COLLAPSED_H;
  /** End of the current burst of arrow-key moves: one undo step per burst. */
  private moveBurstUntil = 0;

  /** True while keyboard focus is inside the canvas. */
  private hasFocus(): boolean {
    const active = document.activeElement;
    return !!active && this.host.nativeElement.contains(active);
  }

  @HostListener('document:keydown', ['$event'])
  onKeyDown(e: KeyboardEvent): void {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement
        || e.target instanceof HTMLSelectElement) return;
    const hit = SHORTCUTS.find(s => s.match(e) && (!s.focus || this.hasFocus()));
    if (!hit) return;
    if (this.readonly() && EDIT_SHORTCUTS.has(hit.id)) return;
    switch (hit.id) {
      case 'undo': e.preventDefault(); this.store.undo(); break;
      case 'redo': e.preventDefault(); this.store.redo(); break;
      case 'select-all': e.preventDefault(); this.store.selectAll(); break;
      case 'group': e.preventDefault(); this.groupSelection(); break;
      case 'tool-select': this.setTool('select'); break;
      case 'tool-pan': this.setTool('pan'); break;
      case 'delete': if (this.editNodeId() === null) this.deleteSelected(); break;
      case 'escape':
        this.cancelEdit(); this.drawing.set(null); this.reconnecting.set(null);
        this.marquee.set(null); this.closeContextMenu(); this.helpOpen.set(false);
        this.store.clearHighlight();
        break;
      case 'zoom-fit': e.preventDefault(); this.zoomToFit(); break;
      case 'zoom-selection': e.preventDefault(); this.zoomToSelection(); break;
      case 'zoom-in': e.preventDefault(); this.zoomIn(); break;
      case 'zoom-out': e.preventDefault(); this.zoomOut(); break;
      case 'search': e.preventDefault(); this.openSearch(); break;
      case 'next-state': e.preventDefault(); this.stepState(1); break;
      case 'prev-state': e.preventDefault(); this.stepState(-1); break;
      case 'next-transition': e.preventDefault(); this.stepTransition(); break;
      case 'rename': {
        const n = this.store.selectedNode();
        if (n) { e.preventDefault(); this.startEdit(n.id, n.label); }
        const a = this.store.selectedAnnotation();
        if (a) { e.preventDefault(); this.editAnnotation(a.id); }
        break;
      }
      case 'move-left': case 'move-right': case 'move-up': case 'move-down': {
        const nodes = this.store.selectedNodes();
        if (!nodes.length) break;
        e.preventDefault();
        const step = e.shiftKey ? GRID * 10 : GRID;
        const dx = hit.id === 'move-left' ? -step : hit.id === 'move-right' ? step : 0;
        const dy = hit.id === 'move-up' ? -step : hit.id === 'move-down' ? step : 0;
        const now = Date.now();
        if (now > this.moveBurstUntil) this.store.checkpoint();
        this.moveBurstUntil = now + 500;
        for (const n of nodes) this.store.moveNode(n.id, n.x + dx, n.y + dy);
        break;
      }
      case 'help': this.helpOpen.update(v => !v); break;
    }
    this.cdr.markForCheck();
  }

  /** Selects and focuses the next (or previous) state in reading order. */
  private stepState(dir: 1 | -1): void {
    const order = readingOrder(this.store.nodes().filter(n => !this.hiddenNodeIds().has(n.id)));
    if (!order.length) return;
    const cur = order.findIndex(n => n.id === this.store.selectedNode()?.id);
    const next = order[cur < 0 ? (dir > 0 ? 0 : order.length - 1) : (cur + dir + order.length) % order.length];
    this.store.select(next.id, 'node');
    this.revealElement(next.id);
    this.focusNode(next.id);
  }

  private stepTransition(): void {
    const edges = this.store.edges().filter(e => this.geometries().has(e.id));
    if (!edges.length) return;
    const cur = edges.findIndex(e => e.id === this.store.selectedEdge()?.id);
    const next = edges[(cur + 1) % edges.length];
    this.store.select(next.id, 'edge');
    this.revealElement(next.id);
  }

  /** Moves keyboard focus to a state's element, after it has rendered. */
  private focusNode(id: string): void {
    setTimeout(() => {
      this.host.nativeElement.querySelector<SVGGElement>(`[data-node-id="${id}"]`)?.focus();
    });
  }

  @HostListener('document:mousedown', ['$event'])
  onDocMouseDown(e: MouseEvent): void {
    // Belt and braces for the focus above: whatever happens to focus, a click
    // outside the label editor commits it, so editing cannot get stuck.
    if (this.editNodeId() !== null
        && !(e.target as Element).closest('.node-edit-input')) {
      this.finishEdit();
    }
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
    // Middle-mouse, Shift/Alt + left, or the hand tool pans. Plain left-drag
    // marquee-selects.
    const wantsPan =
      e.button === 1 || (e.button === 0 && (e.shiftKey || e.altKey || this.tool() === 'pan'));
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
    if (this.readonly()) { this.startPan(e); return; }
    if (e.button !== 0) return;
    if (this.handTool(e)) { this.startPan(e); return; }
    e.stopPropagation();
    if (e.ctrlKey || e.metaKey) this.store.toggleSelect(group.id, 'group');
    else this.store.select(group.id, 'group');
    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    this.groupDrag.set({ groupId: group.id, startX: pt.x, startY: pt.y });
  }

  /** Begin resizing a group from one of its handles. */
  onGroupResizeStart(e: MouseEvent, group: CanvasGroup, handle: ResizeHandle): void {
    if (e.button !== 0) return;
    if (this.handTool(e)) { this.startPan(e); return; }
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

  /**
   * Double-clicking empty canvas centres the diagram in the viewport.
   *
   * It used to drop a new state, which made an easy gesture destructive on a
   * diagram the user was only trying to look at. States are added from the
   * palette, by dragging off a connector dot, or from "Add state here" in the
   * right-click menu — all of which say what they will do first.
   */
  onBgDblClick(e: MouseEvent): void {
    e.preventDefault();
    this.centerView();
  }

  /**
   * Pans so the content sits in the middle of the viewport, leaving the zoom
   * alone. With nothing on the canvas there is nothing to centre, so the view
   * goes back to its default corner.
   */
  /** Size of the canvas on screen; zero in tests and before layout. */
  private viewSize(): { w: number; h: number } {
    const r = this.svgEl()?.nativeElement.getBoundingClientRect();
    return { w: r?.width ?? 0, h: r?.height ?? 0 };
  }

  private applyView(v: View): void {
    this.zoom.set(v.zoom);
    this.panX.set(v.panX);
    this.panY.set(v.panY);
    this.cdr.markForCheck();
  }

  /** Centres the diagram without changing the zoom. */
  centerView(): void {
    const box = boundsOf(this.store.nodes());
    const { w, h } = this.viewSize();
    if (!box || w === 0) { this.resetView(); return; }
    this.applyView(centreOn(box, w, h, this.zoom()));
  }

  /** Zooms and pans so the whole model fills the view (Shift+1). */
  zoomToFit(): void {
    const box = boundsOf(this.store.nodes());
    const { w, h } = this.viewSize();
    if (!box || w === 0) return;
    this.applyView(fitTo(box, w, h));
  }

  /** Zooms and pans so the selected states fill the view (Shift+2). */
  zoomToSelection(): void {
    const boxes = this.store.selection()
      .map(s => this.elementBox(s.id))
      .filter((b): b is Box => !!b);
    const box = boundsOf(boxes);
    const { w, h } = this.viewSize();
    if (!box || w === 0) return;
    this.applyView(fitTo(box, w, h));
  }

  /**
   * Pans, without touching zoom, so the state or transition `id` is on screen.
   * An element already in view is left where it is.
   */
  revealElement(id: string): void {
    // A state inside a collapsed group is shown by expanding the group.
    this.store.expandToShow(id);
    const e = this.store.edgeById(id);
    if (e) { this.store.expandToShow(e.fromId); this.store.expandToShow(e.toId); }
    const box = this.elementBox(id);
    const { w, h } = this.viewSize();
    if (!box || w === 0) return;
    const view = { zoom: this.zoom(), panX: this.panX(), panY: this.panY() };
    if (isVisible(box, view, w, h)) return;
    this.applyView(centreOn(box, w, h, view.zoom));
  }

  /** Canvas-space bounds of a state, or of the two states a transition joins. */
  private elementBox(id: string): { x: number; y: number; w: number; h: number } | null {
    const node = this.store.nodeById(id);
    const rects = node ? [node] : (() => {
      const e = this.store.edgeById(id);
      const ends = e ? [this.store.nodeById(e.fromId), this.store.nodeById(e.toId)] : [];
      return ends.filter((n): n is CanvasNode => !!n);
    })();
    if (rects.length === 0) return null;
    const x0 = Math.min(...rects.map(n => n.x));
    const y0 = Math.min(...rects.map(n => n.y));
    return {
      x: x0, y: y0,
      w: Math.max(...rects.map(n => n.x + n.w)) - x0,
      h: Math.max(...rects.map(n => n.y + n.h)) - y0,
    };
  }

  // ── Node interaction ──────────────────────────────────────────────────
  onNodeMouseDown(e: MouseEvent, node: CanvasNode): void {
    if (e.button !== 0) return;
    // The hand tool pans from anywhere, so grabbing a state moves the view
    // rather than the state.
    if (this.handTool(e)) { this.startPan(e); return; }
    // Present mode: dragging anywhere pans; a click still selects.
    if (this.readonly()) { this.store.select(node.id, 'node'); this.startPan(e); return; }
    e.stopPropagation();
    this.nodeDownAt = { x: e.clientX, y: e.clientY };
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
    const notes = this.store.selectedAnnotations().map(a => ({ id: a.id, x: a.x, y: a.y }));
    this.drag.set({
      nodeId: node.id, startX: pt.x, startY: pt.y,
      origX: node.x, origY: node.y, others, notes,
    });
  }

  // ── Annotations ───────────────────────────────────────────────────────
  /** A note or text box, or one of its handles, was pressed. */
  onAnnotationGrab(g: AnnotationGrab): void {
    const e = g.event;
    const a = g.annotation;
    if (this.readonly()) { this.store.select(a.id, 'annotation'); this.startPan(e); return; }
    if (this.handTool(e)) { this.startPan(e); return; }
    e.stopPropagation();
    const pt = toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
    if (g.kind === 'resize') {
      this.store.select(a.id, 'annotation');
      this.store.checkpoint();
      this.noteResize.set({
        id: a.id, handle: g.handle, startX: pt.x, startY: pt.y,
        orig: { x: a.x, y: a.y, w: a.w, h: a.h },
      });
      return;
    }
    if (e.ctrlKey || e.metaKey) { this.store.toggleSelect(a.id, 'annotation'); return; }
    // Dragging a member of a multi-selection moves the whole selection.
    if (!this.store.isSelectedId(a.id)) this.store.select(a.id, 'annotation');
    this.drag.set({
      nodeId: a.id, isNote: true, startX: pt.x, startY: pt.y, origX: a.x, origY: a.y,
      others: this.store.selectedNodes().map(n => ({ id: n.id, x: n.x, y: n.y })),
      notes: this.store.selectedAnnotations().filter(x => x.id !== a.id).map(x => ({ id: x.id, x: x.x, y: x.y })),
    });
  }

  /** Opens the text editor of an annotation (F2, or after dropping a new one). */
  editAnnotation(id: string): void { this.annotationLayer()?.edit(id); }

  onNodeDblClick(e: MouseEvent, node: CanvasNode): void {
    e.stopPropagation();
    if (this.readonly()) return;
    this.startEdit(node.id, node.label);
  }

  // ── Test case indicators ──────────────────────────────────────────────────
  /** Horizontal distance between two category chips. */
  readonly TEST_CHIP_STEP = TEST_CHIP_STEP;

  /**
   * Where a state's test chips sit: centred under the state, just below its
   * bottom edge.
   *
   * Outside rather than inside, because at a readable size they covered the
   * state's own name — and a shape's interior is not reliably wide where the
   * chips would be anyway, a diamond's lower half least of all.
   */
  testsTransform(node: CanvasNode): string {
    const width = this.testsRowWidth(node);
    const x = Math.round((node.w - width) / 2);
    return `translate(${x},${node.h + 13})`;
  }

  /** Width of a state's whole chip row. */
  testsRowWidth(node: CanvasNode): number {
    const count = this.store.testCounts(node).length;
    return count > 0 ? (count - 1) * TEST_CHIP_STEP + TEST_CHIP_W : 0;
  }

  /**
   * Double-clicking the chips reveals the state's test cases instead of
   * renaming the state, which is what a double-click elsewhere on it does.
   */
  onTestsDblClick(e: MouseEvent, node: CanvasNode): void {
    e.stopPropagation();
    e.preventDefault();
    this.revealTestsFor(node);
  }

  revealTestsFor(node: CanvasNode): void {
    this.store.select(node.id, 'node');
    this.revealTests.emit();
  }

  /** Asks the page to open the properties panel on this state's test cases. */
  readonly revealTests = output<void>();

  /** The inline label editor, while one is open. */
  private readonly editInput = viewChild<ElementRef<HTMLTextAreaElement>>('editInput');

  // ── Connector (start drawing edge) ────────────────────────────────────
  onConnectorMouseDown(e: MouseEvent, node: CanvasNode, anchor: Anchor): void {
    if (this.readonly()) return;
    if (this.handTool(e)) { this.startPan(e); return; }
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
    // Alt+click on the line adds a bend point there.
    if (e.altKey && !this.readonly()) {
      this.addBendPoint(edge.id, toCanvas(e, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY()));
    }
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
    const g = this.geometries().get(e.id);
    return g ? (end === 'from' ? g.src : g.tgt) : null;
  }

  /** States folded into a collapsed group. */
  readonly hiddenNodeIds = computed(() => new Set(this.store.hiddenBy().keys()));
  readonly visibleNodes = computed(() => {
    const hidden = this.hiddenNodeIds();
    return this.store.nodes().filter(n => !hidden.has(n.id));
  });

  /**
   * Path, label and handle points of every visible transition (see
   * edge-geometry). An end inside a collapsed group attaches to the group's
   * box; a transition with both ends in the same collapsed group is hidden.
   */
  readonly geometries = computed(() => {
    const hiddenBy = this.store.hiddenBy();
    const end = (id: string) => {
      const g = hiddenBy.get(id);
      return g ? collapsedNode(g) : this.store.nodeById(id);
    };
    const loops = loopIndices(this.store.edges());
    const out = new Map<string, EdgeGeometry>();
    for (const e of this.store.edges()) {
      const gf = hiddenBy.get(e.fromId), gt = hiddenBy.get(e.toId);
      if (gf && gf === gt) continue;
      const from = end(e.fromId);
      const to = end(e.toId);
      if (!from || !to) continue;
      // Waypoints belong to the full layout; a folded end gets a plain line.
      const shown = gf || gt ? { ...e, waypoints: undefined, fromAnchor: gf ? undefined : e.fromAnchor,
        toAnchor: gt ? undefined : e.toAnchor } : e;
      out.set(e.id, edgeGeometry(shown, from, to, loops.get(e.id) ?? 0));
    }
    return out;
  });

  // ── Routing drags: bend handle, waypoints, label ──────────────────────
  private readonly routingDrag = signal<
    | { kind: 'bend'; edgeId: string }
    | { kind: 'waypoint'; edgeId: string; index: number }
    | { kind: 'label'; edgeId: string; startX: number; startY: number; dx: number; dy: number }
    | null
  >(null);

  /** A handle of a transition was pressed: start the matching drag. */
  onEdgeGrab(g: EdgeGrab): void {
    if (this.readonly()) return;
    if (this.handTool(g.event)) { this.startPan(g.event); return; }
    if (g.kind === 'endpoint') { this.onEndpointMouseDown(g.event, g.edge, g.end); return; }
    this.store.select(g.edge.id, 'edge');
    this.store.checkpoint();
    if (g.kind === 'bend') {
      this.routingDrag.set({ kind: 'bend', edgeId: g.edge.id });
    } else if (g.kind === 'waypoint') {
      this.routingDrag.set({ kind: 'waypoint', edgeId: g.edge.id, index: g.index });
    } else {
      const pt = toCanvas(g.event, this.svgEl().nativeElement, this.zoom(), this.panX(), this.panY());
      const o = g.edge.labelOffset ?? { dx: 0, dy: 0 };
      this.routingDrag.set({ kind: 'label', edgeId: g.edge.id, startX: pt.x, startY: pt.y, dx: o.dx, dy: o.dy });
    }
  }

  private moveRoutingDrag(pt: { x: number; y: number }): void {
    const r = this.routingDrag();
    if (!r) return;
    if (r.kind === 'bend') {
      const g = this.geometries().get(r.edgeId);
      if (g) this.store.setEdgeCurveLive(r.edgeId, curveThrough(g.src, g.tgt, pt));
    } else if (r.kind === 'waypoint') {
      this.store.moveWaypointLive(r.edgeId, r.index, pt);
    } else {
      this.store.setLabelOffsetLive(r.edgeId, r.dx + pt.x - r.startX, r.dy + pt.y - r.startY);
    }
  }

  /** Adds a bend point at `pt`, on the segment of the route nearest to it. */
  addBendPoint(edgeId: string, pt: { x: number; y: number }): void {
    const e = this.store.edgeById(edgeId);
    const g = this.geometries().get(edgeId);
    if (!e || !g || e.fromId === e.toId) return;
    this.store.addWaypoint(edgeId, insertIndex(g.src, e.waypoints ?? [], g.tgt, pt), pt);
  }

  /** Grab one end of a transition to re-attach it elsewhere. */
  onEndpointMouseDown(ev: MouseEvent, edge: CanvasEdge, end: 'from' | 'to'): void {
    if (this.handTool(ev)) { this.startPan(ev); return; }
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

    const nr = this.noteResize();
    if (nr) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      this.store.resizeAnnotation(nr.id, nr.handle, nr.orig, pt.x - nr.startX, pt.y - nr.startY);
      this.cdr.markForCheck();
      return;
    }

    const d = this.drag();
    if (d) {
      const pt = toCanvas(e, svg, this.zoom(), this.panX(), this.panY());
      let dx = pt.x - d.startX, dy = pt.y - d.startY;
      // The whole move is one undo step, taken once it really moves.
      if (!d.moved && (dx || dy)) { this.store.checkpoint(); this.drag.set({ ...d, moved: true }); }
      // Snap the grabbed element; the rest of the selection keeps its offsets.
      const box = d.isNote ? this.store.annotationById(d.nodeId) : this.store.nodeById(d.nodeId);
      if (box && this.snapOn() && !e.altKey) {
        const moved = new Set([d.nodeId, ...d.others.map(o => o.id), ...d.notes.map(o => o.id)]);
        const snapped = snapPosition(
          { x: d.origX + dx, y: d.origY + dy, w: box.w, h: box.h },
          [...this.visibleNodes(), ...this.store.annotations()].filter(n => !moved.has(n.id)),
          6 / this.zoom(),
        );
        dx = snapped.x - d.origX;
        dy = snapped.y - d.origY;
        this.guides.set(snapped.guides);
      } else {
        this.guides.set([]);
      }
      if (d.isNote) this.store.moveAnnotation(d.nodeId, d.origX + dx, d.origY + dy);
      else this.store.moveNode(d.nodeId, d.origX + dx, d.origY + dy);
      for (const o of d.others) this.store.moveNode(o.id, o.x + dx, o.y + dy);
      for (const o of d.notes) this.store.moveAnnotation(o.id, o.x + dx, o.y + dy);
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

    if (this.routingDrag()) {
      this.moveRoutingDrag(toCanvas(e, svg, this.zoom(), this.panX(), this.panY()));
      this.cdr.markForCheck();
    }
  }

  @HostListener('document:mouseup', ['$event'])
  onDocMouseUp(e: MouseEvent): void {
    const mq = this.marquee();
    if (mq) {
      const hits = this.store.nodesInRect(mq.x0, mq.y0, mq.x1, mq.y1);
      const notes = this.store.annotationsInRect(mq.x0, mq.y0, mq.x1, mq.y1);
      // A click with no drag just clears; a drag selects what it covered.
      const dragged = Math.abs(mq.x1 - mq.x0) > 3 || Math.abs(mq.y1 - mq.y0) > 3;
      if (dragged) {
        if (mq.additive) {
          for (const id of hits) if (!this.store.isSelectedId(id)) this.store.toggleSelect(id, 'node');
          for (const id of notes) if (!this.store.isSelectedId(id)) this.store.toggleSelect(id, 'annotation');
        } else {
          this.store.selectItems(hits, notes);
        }
      }
      this.marquee.set(null);
    }

    if (this.drag()) this.drag.set(null);
    this.noteResize.set(null);
    this.guides.set([]);
    this.routingDrag.set(null);
    this.groupDrag.set(null);
    this.groupResize.set(null);
    this.panning.set(null);

    const draw = this.drawing();
    if (draw) {
      // A node's own (mouseup) clears `drawing` before this runs, so if it is
      // still set the transition was released on empty canvas: create the
      // target state there and connect it.
      const svg = this.svgEl()?.nativeElement;
      const pt = svg ? toCanvas(e, svg, this.zoom(), this.panX(), this.panY()) : null;
      // On a note or text box: no transition, and no new state under it.
      const onNote = !!pt && this.store.annotationsInRect(pt.x, pt.y, pt.x, pt.y).length > 0;
      if (svg && pt && !onNote && this.isInsideCanvas(e, svg)) {
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
    this.editOriginal = label;
    this.cdr.markForCheck();
    // Focus the input once it exists. Without this nothing has focus, so no
    // blur ever fires, editing never ends, and every shortcut that is
    // suppressed during editing — Delete above all — stays dead.
    setTimeout(() => {
      const el = this.editInput()?.nativeElement;
      if (!el) return;
      el.focus();
      el.select();
    });
  }

  finishEdit(): void {
    const id = this.editNodeId();
    if (!id) return;
    // trim() also drops trailing blank lines.
    const val = this.editValue().trim() || 'State';
    this.store.updateNode(id, { label: val });
    this.editNodeId.set(null);
  }

  cancelEdit(): void {
    const id = this.editNodeId();
    if (!id) return;
    this.store.updateNode(id, { label: this.editOriginal });
    this.editNodeId.set(null);
  }

  /** Typing updates the name as it goes, so the state resizes live. */
  onEditInput(value: string): void {
    this.editValue.set(value);
    const id = this.editNodeId();
    if (id) this.store.updateNode(id, { label: value });
  }

  onEditKeydown(e: KeyboardEvent): void {
    // Shift+Enter falls through to the textarea and inserts a line break.
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); this.finishEdit(); }
    if (e.key === 'Escape') { e.preventDefault(); this.cancelEdit(); }
  }

  // ── Path helpers ──────────────────────────────────────────────────────
  /** Endpoints of an edge: its dots when set, else centre-to-centre geometry. */
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
    // One line is 28px; each further line adds a line height, within the state.
    const lines = labelLines(n.label, n.shape, n.style).length;
    const h = Math.min(Math.max(28, lines * labelLineH(n.style) + 12), n.h - 4);
    // Fraction of the width that stays inside the outline at mid-height.
    const frac = innerWidthFrac(n.shape);
    const w = frac === 1 ? n.w - 12 : Math.round(n.w * frac);
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
