import {
  afterNextRender, ChangeDetectionStrategy, Component, computed, DestroyRef, effect, ElementRef,
  HostListener, inject, signal, untracked, viewChild,
} from '@angular/core';
import {
  ArchComponent, ArchComponentType, Flow, RiskLevel,
} from '../../../../core/api/threat-model-api';
import {
  Anchor, anchorPoint, ANCHORS, CanvasEdge, CanvasNode, ElementStyle, nearestAnchor, NodeShape,
} from '../../../models/state/model-editor.store';
import { curveThrough, edgeGeometry, facingSide } from '../../../models/state/edge-geometry';
import { ALL_SHAPES, shapeIcon, shapePath } from '../../../models/state/node-shapes';
import { snapPosition } from '../../../models/state/snapping';
import {
  arrowMarkerId, InlineStyle, linePaint, shapePaint, textPaint,
} from '../../../models/state/style-render';
import { boundsOf, fitTo, MAX_ZOOM, MIN_ZOOM, View } from '../../../models/state/viewport';
import { CanvasMinimapComponent } from '../../../models/components/canvas-minimap/canvas-minimap';
import { StyleChange, StyleTarget } from '../../../models/components/style-section/style-section';
import {
  DiagramLayout, EdgeLayout, exitPoint, fitAround, NODE_H, NODE_W, parseLayout, resolveLayout,
  zoneAt,
} from '../../state/diagram-layout';
import { componentInput, flowInput } from '../../state/inputs';
import { errorMessage, ThreatModelStore } from '../../state/threat-model.store';
import {
  freeSpot, highestLevel, label, layerOf, LEVEL_TEXT, openThreatsOn, Rect, TYPE_GROUPS,
} from '../../state/threat-model-view';
import { WorkspaceNav } from '../../state/workspace-nav';
import { ComponentsTabComponent } from '../components-tab/components-tab';
import { DiagramPropertiesComponent, SingleInfo } from '../diagram-properties/diagram-properties';
import { FlowsTabComponent } from '../flows-tab/flows-tab';
import { ZonesTabComponent } from '../zones-tab/zones-tab';

type Kind = 'node' | 'edge' | 'zone';
interface Sel { kind: Kind; id: string }
interface Menu { kind: Kind | 'bg'; id?: string; sx: number; sy: number; at: { x: number; y: number } }
type Drag =
  | { mode: 'pan'; sx: number; sy: number; view: View }
  | { mode: 'node'; id: string; dx: number; dy: number; moved: boolean; start: DiagramLayout }
  | { mode: 'zone'; id: string; sx: number; sy: number; start: DiagramLayout; moved: boolean }
  | { mode: 'resize'; id: string; sx: number; sy: number; start: Rect }
  | { mode: 'connect'; from: string; anchor: Anchor }
  | { mode: 'end'; flow: string; end: 'source' | 'target' }
  | { mode: 'bend'; flow: string };

/** The shape a component is drawn with by default, by layer. */
export function shapeFor(t: ArchComponentType): 'box' | NodeShape {
  const layer = layerOf(t);
  if (layer === 'external') return 'box';
  if (layer === 'data') return 'cylinder';
  return 'rect';
}

const LAYER_ICON: Record<string, string> = {
  client: '▭', integration: '⇄', logic: '⚙', data: '⛁', 'identity-security': '◈',
  platform: '☁', external: '☺',
};

const SAVE_DELAY_MS = 600;
const EDGE_COLOUR = 'var(--clr-edge)';
const SELECTED_COLOUR = 'var(--clr-brand-500)';
const NAME_PX = 13;
const LABEL_PX = 11;

/** An arrowhead the template defines: its kind in one colour. */
interface Marker { id: string; kind: 'filled' | 'open' | 'line'; colour: string }

/**
 * The data-flow diagram (06-ui.md, threat-model-diagram spec): components as
 * shapes, flows as arrows between connector dots, zones as regions. It draws
 * and styles like the model editor's canvas; every structural edit is the same
 * entity change the tables make, and everything visual is saved as the layout.
 */
@Component({
  selector: 'tm-threat-diagram',
  imports: [
    CanvasMinimapComponent, ComponentsTabComponent, FlowsTabComponent, ZonesTabComponent,
    DiagramPropertiesComponent,
  ],
  templateUrl: './threat-diagram.html',
  styleUrl: './threat-diagram.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ThreatDiagramComponent {
  readonly store = inject(ThreatModelStore);
  private readonly nav = inject(WorkspaceNav);
  private readonly svg = viewChild.required<ElementRef<SVGSVGElement>>('svg');
  private readonly menuEl = viewChild<ElementRef<HTMLElement>>('menuEl');
  private readonly componentsEditor = viewChild.required(ComponentsTabComponent);
  private readonly flowsEditor = viewChild.required(FlowsTabComponent);
  private readonly zonesEditor = viewChild.required(ZonesTabComponent);

  readonly groups = TYPE_GROUPS;
  readonly label = label;
  readonly levelText = LEVEL_TEXT;
  readonly icon = LAYER_ICON;
  readonly anchors = ANCHORS;
  readonly shapes = ALL_SHAPES;
  readonly shapeIcon = shapeIcon;

  readonly view = signal<View>({ zoom: 1, panX: 0, panY: 0 });
  readonly size = signal({ w: 0, h: 0 });
  readonly selection = signal<Sel[]>([]);
  readonly overlay = signal(false);
  readonly newType = signal<ArchComponentType>('service');
  readonly message = signal<string | null>(null);
  readonly menu = signal<Menu | null>(null);
  /** The stand-alone Properties panel, for several selected elements (right-click, Properties…). */
  readonly showProps = signal(false);
  /** The user's layout since the last load; null means the stored one. */
  private readonly local = signal<DiagramLayout | null>(null);
  /** The pointer while drawing a flow or moving an end (canvas units). */
  readonly pointer = signal<{ x: number; y: number } | null>(null);
  /** Where the ghost line starts: the pressed dot, or the flow's other end. */
  readonly ghostFrom = signal<{ x: number; y: number } | null>(null);
  private drag: Drag | null = null;
  private saveTimer: ReturnType<typeof setTimeout> | null = null;

  readonly layout = computed(() => resolveLayout(
    this.local() ?? parseLayout(this.store.model()?.layout),
    this.store.components(), this.store.zones(), this.store.flows()));

  /** Components in reading order, so Tab moves top to bottom, left to right. */
  readonly nodes = computed(() => {
    const l = this.layout();
    return this.store.components()
      .map(c => ({ c, r: l.nodes[c.id], cn: this.canvasNode(c, l) }))
      .sort((a, b) => a.r.y - b.r.y || a.r.x - b.r.x);
  });

  readonly zones = computed(() => {
    const l = this.layout();
    return this.store.zones().map(z => ({ z, r: l.zones[z.id] }));
  });

  readonly edges = computed(() => {
    const l = this.layout();
    const byId = new Map(this.store.components().map(c => [c.id, c]));
    return this.store.flows().flatMap(f => {
      const a = byId.get(f.sourceId), b = byId.get(f.targetId);
      if (!a || !b) return [];
      const from = this.canvasNode(a, l), to = this.canvasNode(b, l);
      const e = this.canvasEdge(f, from, to, l.edges[f.id] ?? {});
      const g = edgeGeometry(e, from, to);
      return [{ f, e, g, name: `${a.name} → ${b.name}`, marker: this.marker(f, a, b, l, g) }];
    });
  });

  /** Arrowheads in use: one per kind and colour, like the model editor. */
  readonly markers = computed<Marker[]>(() => {
    const out = new Map<string, Marker>();
    const add = (kind: Marker['kind'], colour: string, key: string) => {
      const id = arrowMarkerId(kind, `dg${key}`);
      if (!out.has(id)) out.set(id, { id, kind, colour });
    };
    for (const { e } of this.edges()) {
      const kind = e.style?.arrow ?? 'filled';
      add(kind, e.style?.stroke ?? EDGE_COLOUR, e.style?.stroke ?? 'default');
      if (this.isSelected(e.id)) add(kind, SELECTED_COLOUR, 'selected');
    }
    return [...out.values()];
  });

  readonly boxes = computed(() => [
    ...Object.values(this.layout().nodes), ...Object.values(this.layout().zones),
  ]);

  readonly transform = computed(() => {
    const v = this.view();
    return `translate(${v.panX} ${v.panY}) scale(${v.zoom})`;
  });

  /** The single selected flow, for its end and bend handles. */
  readonly selectedEdge = computed(() => {
    const s = this.selection();
    return s.length === 1 && s[0].kind === 'edge' ? this.edges().find(x => x.f.id === s[0].id) ?? null : null;
  });

  // ── Properties ────────────────────────────────────────────────────────

  readonly styleTargets = computed<StyleTarget[]>(() => {
    const l = this.layout();
    return this.selection().map(s => ({
      kind: s.kind === 'edge' ? 'edge' : 'node',
      style: s.kind === 'node' ? l.nodes[s.id]?.style : s.kind === 'zone' ? l.zones[s.id]?.style : l.edges[s.id]?.style,
    }));
  });

  readonly single = computed<SingleInfo | null>(() => {
    const s = this.selection();
    if (s.length !== 1) return null;
    const l = this.layout();
    const [x] = s;
    if (x.kind === 'node') return { kind: 'node', shape: l.nodes[x.id]?.shape ?? null };
    if (x.kind === 'edge') {
      const e = l.edges[x.id] ?? {};
      return { kind: 'edge', routing: e.routing ?? 'curved', curved: !!e.curve };
    }
    return { kind: 'zone' };
  });

  readonly propsHeading = computed(() => {
    const s = this.selection();
    if (s.length !== 1) return `${s.length} elements`;
    return this.nameOf(s[0]);
  });

  constructor() {
    const destroy = inject(DestroyRef);
    afterNextRender(() => {
      const el = this.svg().nativeElement;
      if (typeof ResizeObserver !== 'undefined') {
        const ro = new ResizeObserver(([e]) => {
          const first = !this.size().w;
          this.size.set({ w: e.contentRect.width, h: e.contentRect.height });
          if (first) this.fit();
        });
        ro.observe(el);
        destroy.onDestroy(() => ro.disconnect());
      }
    });
    destroy.onDestroy(() => this.flushSave());
    // A newer layout from the server replaces the local one once saved.
    effect(() => {
      this.store.model();
      untracked(() => { if (!this.saveTimer) this.local.set(null); });
    });
    // Focus the first menu item when a menu opens.
    effect(() => {
      if (!this.selection().length) untracked(() => { this.showProps.set(false); this.closeDetails(); });
    });
    effect(() => {
      if (this.menu()) queueMicrotask(() => this.menuEl()?.nativeElement.querySelector<HTMLElement>('[role="menuitem"]')?.focus());
    });
  }

  // ── Geometry ──────────────────────────────────────────────────────────

  private canvasNode(c: ArchComponent, l: DiagramLayout): CanvasNode {
    const n = l.nodes[c.id];
    const s = n?.shape ?? shapeFor(c.type);
    return {
      id: c.id, label: c.name, kind: 'regular', x: n.x, y: n.y, w: n.w, h: n.h, tests: [],
      shape: s === 'box' ? 'rect' : s, style: n.style,
    };
  }

  /** A flow without stored dots joins the facing sides, never the centres. */
  private canvasEdge(f: Flow, from: CanvasNode, to: CanvasNode, e: EdgeLayout): CanvasEdge {
    const centre = (n: CanvasNode) => ({ x: n.x + n.w / 2, y: n.y + n.h / 2 });
    return {
      id: f.id, fromId: from.id, toId: to.id, label: f.label ?? '',
      curve: e.curve ?? 0, routing: e.routing, style: e.style,
      fromAnchor: e.fromAnchor ?? facingSide(from, centre(to)),
      toAnchor: e.toAnchor ?? facingSide(to, centre(from)),
    };
  }

  private centre(r: Rect) {
    return { x: r.x + r.w / 2, y: r.y + r.h / 2 };
  }

  /** Where a crossing flow leaves the source's zone (or enters the target's). */
  private marker(f: Flow, a: ArchComponent, b: ArchComponent, l: DiagramLayout, g: { src: { x: number; y: number }; tgt: { x: number; y: number } }) {
    if (!f.crossing.crosses) return null;
    if (a.zoneId && l.zones[a.zoneId]) {
      const p = exitPoint(g.src, g.tgt, l.zones[a.zoneId]) ?? exitPoint(this.centre(l.nodes[a.id]), g.tgt, l.zones[a.zoneId]);
      if (p) return p;
    }
    if (b.zoneId && l.zones[b.zoneId]) return exitPoint(g.tgt, g.src, l.zones[b.zoneId]) ?? exitPoint(this.centre(l.nodes[b.id]), g.src, l.zones[b.zoneId]);
    return null;
  }

  isBox(n: { c: ArchComponent; cn: CanvasNode }): boolean {
    return !this.layout().nodes[n.c.id]?.shape && shapeFor(n.c.type) === 'box';
  }

  shapeD(n: { cn: CanvasNode }): string {
    return shapePath(n.cn.shape, n.cn.w, n.cn.h);
  }

  isData(n: { c: ArchComponent; cn: CanvasNode }): boolean {
    return n.cn.shape === 'cylinder';
  }

  dot(n: { cn: CanvasNode }, a: Anchor) {
    const p = anchorPoint(n.cn, a);
    return { x: p.x - n.cn.x, y: p.y - n.cn.y };
  }

  nodePaint(n: { cn: CanvasNode }): InlineStyle { return shapePaint(n.cn.style, 1.5); }
  namePaint(n: { cn: CanvasNode }): InlineStyle { return textPaint(n.cn.style, NAME_PX); }
  edgePaint(e: CanvasEdge): InlineStyle { return linePaint(e.style, 1.5); }
  edgeLabelPaint(e: CanvasEdge): InlineStyle { return textPaint(e.style, LABEL_PX); }
  zonePaint(id: string): InlineStyle { return shapePaint(this.layout().zones[id]?.style, 1); }
  zoneLabelPaint(id: string): InlineStyle { return textPaint(this.layout().zones[id]?.style, 12); }

  /** Width of a flow label's background: an estimate at the label's 11px font. */
  labelWidth(text: string): number {
    return Math.round(text.length * 6.2 + 12);
  }

  markerOf(e: CanvasEdge): string {
    const kind = e.style?.arrow ?? 'filled';
    if (this.isSelected(e.id)) return arrowMarkerId(kind, 'dgselected');
    return arrowMarkerId(kind, `dg${e.style?.stroke ?? 'default'}`);
  }

  /** Open threats on an element and their highest current level. */
  badge(id: string): { count: number; level: RiskLevel | null } | null {
    const ts = openThreatsOn(id, this.store.threats());
    return ts.length ? { count: ts.length, level: highestLevel(ts.map(t => t.currentLevel)) } : null;
  }

  zoneLabel(id: string | null | undefined): string {
    return this.store.zones().find(z => z.id === id)?.name ?? 'no zone';
  }

  edgeLabel(e: { f: Flow; name: string }): string {
    return `Flow ${e.name}${e.f.label ? ', ' + e.f.label : ''}${e.f.entryPoint ? ', entry point' : ''}${e.f.crossing.crosses ? ', crosses trust boundary' : ''}`;
  }

  /** The label drawn on a flow: its own text, prefixed with ⇥ for an entry point. */
  edgeText(f: Flow): string {
    return (f.entryPoint ? '⇥ ' : '') + (f.label ?? '');
  }

  private nameOf(s: Sel): string {
    if (s.kind === 'node') return this.store.components().find(c => c.id === s.id)?.name ?? '';
    if (s.kind === 'zone') return `Zone ${this.zoneLabel(s.id)}`;
    return this.edges().find(e => e.f.id === s.id)?.name ?? 'Flow';
  }

  menuTitle(m: Menu): string {
    return m.kind === 'bg' ? 'Diagram' : this.nameOf({ kind: m.kind, id: m.id! });
  }

  private toCanvas(ev: { clientX: number; clientY: number }) {
    const rect = this.svg().nativeElement.getBoundingClientRect();
    const v = this.view();
    return { x: (ev.clientX - rect.left - v.panX) / v.zoom, y: (ev.clientY - rect.top - v.panY) / v.zoom };
  }

  private nodeAt(p: { x: number; y: number }) {
    return this.nodes().find(n => p.x >= n.r.x - 6 && p.x <= n.r.x + n.r.w + 6 && p.y >= n.r.y - 6 && p.y <= n.r.y + n.r.h + 6);
  }

  // ── Selection ─────────────────────────────────────────────────────────

  isSelected(id: string): boolean {
    return this.selection().some(s => s.id === id);
  }

  private select(s: Sel, additive: boolean): void {
    this.menu.set(null);
    this.closeDetails();
    if (!additive) { this.selection.set([s]); return; }
    this.selection.update(cur => cur.some(x => x.id === s.id) ? cur.filter(x => x.id !== s.id) : [...cur, s]);
  }

  // ── View ──────────────────────────────────────────────────────────────

  fit(): void {
    const b = boundsOf(this.boxes());
    const { w, h } = this.size();
    if (b && w && h) this.view.set(fitTo(b, w, h));
  }

  zoomBy(f: number, at?: { x: number; y: number }): void {
    const v = this.view();
    const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, v.zoom * f));
    const { w, h } = this.size();
    const p = at ?? { x: w / 2, y: h / 2 };
    this.view.set({ zoom, panX: p.x - (p.x - v.panX) * (zoom / v.zoom), panY: p.y - (p.y - v.panY) * (zoom / v.zoom) });
  }

  panTo(p: { x: number; y: number }): void {
    const { w, h } = this.size();
    const z = this.view().zoom;
    this.view.set({ zoom: z, panX: w / 2 - p.x * z, panY: h / 2 - p.y * z });
  }

  onWheel(e: WheelEvent): void {
    e.preventDefault();
    const rect = this.svg().nativeElement.getBoundingClientRect();
    this.zoomBy(e.deltaY < 0 ? 1.1 : 1 / 1.1, { x: e.clientX - rect.left, y: e.clientY - rect.top });
  }

  // ── Pointer ───────────────────────────────────────────────────────────

  onBackgroundDown(e: PointerEvent): void {
    if (e.button !== 0 && e.button !== 1) return;
    this.menu.set(null);
    if (!e.shiftKey) this.selection.set([]);
    this.drag = { mode: 'pan', sx: e.clientX, sy: e.clientY, view: this.view() };
  }

  onNodeDown(e: PointerEvent, c: ArchComponent): void {
    e.stopPropagation();
    if (e.button !== 0) return;
    if (!(e.shiftKey && this.isSelected(c.id))) this.select({ kind: 'node', id: c.id }, e.shiftKey);
    const p = this.toCanvas(e), r = this.layout().nodes[c.id];
    this.drag = { mode: 'node', id: c.id, dx: p.x - r.x, dy: p.y - r.y, moved: false, start: structuredClone(this.layout()) };
  }

  onEdgeDown(e: PointerEvent, f: Flow): void {
    e.stopPropagation();
    if (e.button !== 0) return;
    this.select({ kind: 'edge', id: f.id }, e.shiftKey);
    this.openDetail('details');
  }

  onDotDown(e: PointerEvent, c: ArchComponent, anchor: Anchor): void {
    e.stopPropagation();
    if (e.button !== 0) return;
    this.drag = { mode: 'connect', from: c.id, anchor };
    const n = this.nodes().find(x => x.c.id === c.id);
    this.ghostFrom.set(n ? anchorPoint(n.cn, anchor) : null);
    this.pointer.set(this.toCanvas(e));
  }

  onEndDown(e: PointerEvent, f: Flow, end: 'source' | 'target'): void {
    e.stopPropagation();
    this.drag = { mode: 'end', flow: f.id, end };
    const g = this.edges().find(x => x.f.id === f.id)?.g;
    this.ghostFrom.set(g ? (end === 'source' ? g.tgt : g.src) : null);
    this.pointer.set(this.toCanvas(e));
  }

  onBendDown(e: PointerEvent, f: Flow): void {
    e.stopPropagation();
    this.drag = { mode: 'bend', flow: f.id };
  }

  onZoneDown(e: PointerEvent, id: string): void {
    e.stopPropagation();
    if (e.button !== 0) return;
    this.select({ kind: 'zone', id }, e.shiftKey);
    this.drag = { mode: 'zone', id, sx: e.clientX, sy: e.clientY, start: structuredClone(this.layout()), moved: false };
  }

  onResizeDown(e: PointerEvent, id: string): void {
    e.stopPropagation();
    this.drag = { mode: 'resize', id, sx: e.clientX, sy: e.clientY, start: { ...this.layout().zones[id] } };
  }

  onMove(e: PointerEvent): void {
    const d = this.drag;
    if (!d) return;
    const z = this.view().zoom;
    switch (d.mode) {
      case 'pan':
        this.view.set({ ...d.view, panX: d.view.panX + e.clientX - d.sx, panY: d.view.panY + e.clientY - d.sy });
        break;
      case 'node': {
        // Moves every selected component together.
        const p = this.toCanvas(e);
        const l = structuredClone(d.start);
        const r = d.start.nodes[d.id];
        const ids = this.selection().filter(s => s.kind === 'node').map(s => s.id);
        const others = Object.entries(d.start.nodes).filter(([id]) => !ids.includes(id)).map(([, b]) => b);
        const s = e.altKey ? { x: p.x - d.dx, y: p.y - d.dy } : snapPosition({ ...r, x: p.x - d.dx, y: p.y - d.dy }, others, 6 / z);
        const dx = s.x - r.x, dy = s.y - r.y;
        for (const id of ids.includes(d.id) ? ids : [d.id]) {
          const n = d.start.nodes[id];
          if (n) l.nodes[id] = { ...n, x: n.x + dx, y: n.y + dy };
        }
        d.moved = true;
        this.local.set(l);
        break;
      }
      case 'zone': {
        const dx = (e.clientX - d.sx) / z, dy = (e.clientY - d.sy) / z;
        if (Math.abs(dx) + Math.abs(dy) > 2) d.moved = true;
        const l = structuredClone(d.start);
        const zr = l.zones[d.id];
        l.zones[d.id] = { ...zr, x: zr.x + dx, y: zr.y + dy };
        for (const c of this.store.components()) {
          if (c.zoneId === d.id) l.nodes[c.id] = { ...l.nodes[c.id], x: l.nodes[c.id].x + dx, y: l.nodes[c.id].y + dy };
        }
        this.local.set(l);
        break;
      }
      case 'resize': {
        const l = this.layout();
        const w = Math.max(120, d.start.w + (e.clientX - d.sx) / z);
        const h = Math.max(80, d.start.h + (e.clientY - d.sy) / z);
        this.local.set({ ...l, zones: { ...l.zones, [d.id]: { ...l.zones[d.id], ...d.start, w, h } } });
        break;
      }
      case 'bend': {
        const ed = this.edges().find(x => x.f.id === d.flow);
        if (!ed) break;
        const curve = Math.round(curveThrough(ed.g.src, ed.g.tgt, this.toCanvas(e)));
        this.patchEdge(d.flow, { curve: Math.abs(curve) < 4 ? 0 : curve }, false);
        break;
      }
      case 'connect':
      case 'end':
        this.pointer.set(this.toCanvas(e));
        break;
    }
  }

  async onUp(e: PointerEvent): Promise<void> {
    const d = this.drag;
    this.drag = null;
    if (!d) return;
    const p = this.toCanvas(e);
    this.ghostFrom.set(null);
    if (d.mode === 'node' && !d.moved) {
      this.openDetail('details');
    } else if (d.mode === 'node') {
      this.scheduleSave();
      for (const s of this.selection().filter(x => x.kind === 'node')) await this.dropInZone(s.id);
      if (!this.isSelected(d.id)) await this.dropInZone(d.id);
    } else if (d.mode === 'zone' && !d.moved) {
      this.openDetail('details');
    } else if (d.mode === 'zone' || d.mode === 'resize' || d.mode === 'bend') {
      this.scheduleSave();
    } else if (d.mode === 'connect') {
      this.pointer.set(null);
      const target = this.nodeAt(p);
      if (target && target.c.id !== d.from) await this.connect(d.from, d.anchor, target.c.id, nearestAnchor(target.cn, p.x, p.y));
    } else if (d.mode === 'end') {
      this.pointer.set(null);
      await this.reattach(d.flow, d.end, p);
    }
  }

  /** Dropping a component into or out of a region changes its zone. */
  private async dropInZone(id: string): Promise<void> {
    const c = this.store.components().find(x => x.id === id);
    const r = this.layout().nodes[id];
    if (!c || !r) return;
    const zone = zoneAt(this.centre(r), this.layout());
    if ((zone ?? null) === (c.zoneId ?? null)) return;
    await this.setZone(c, zone);
  }

  private async setZone(c: ArchComponent, zone: string | null): Promise<void> {
    try {
      await this.store.save('components', { ...componentInput(c), zoneId: zone }, c);
      this.message.set(`${c.name} is now in ${this.zoneLabel(zone)}.`);
    } catch (err) {
      this.message.set(errorMessage(err));
    }
  }

  private async connect(from: string, fromAnchor: Anchor, to: string, toAnchor: Anchor): Promise<void> {
    try {
      const f = await this.store.save('flows', {
        sourceId: from, targetId: to, relationship: 'calls', bidirectional: false,
        authentication: 'unknown', encrypted: 'unknown', assetIds: [],
      });
      this.patchEdge(f.id, { fromAnchor, toAnchor });
      const fresh = this.store.flows().find(x => x.id === f.id) ?? f;
      this.selection.set([{ kind: 'edge', id: f.id }]);
      this.closeDetails();
      this.flowsEditor().drawerTab.set('details');
      this.flowsEditor().edit(fresh);
    } catch (err) {
      this.message.set(errorMessage(err));
    }
  }

  /** Moves a flow's end to the dot nearest `p`; another component changes the flow. */
  private async reattach(flowId: string, end: 'source' | 'target', p: { x: number; y: number }): Promise<void> {
    const f = this.store.flows().find(x => x.id === flowId);
    const target = this.nodeAt(p);
    if (!f || !target) return;
    const anchor = nearestAnchor(target.cn, p.x, p.y);
    const current = end === 'source' ? f.sourceId : f.targetId;
    const other = end === 'source' ? f.targetId : f.sourceId;
    if (target.c.id !== current) {
      if (target.c.id === other) return;
      try {
        await this.store.save('flows', { ...flowInput(f), [end === 'source' ? 'sourceId' : 'targetId']: target.c.id }, f);
      } catch (err) {
        this.message.set(errorMessage(err));
        return;
      }
    }
    this.patchEdge(flowId, end === 'source' ? { fromAnchor: anchor } : { toAnchor: anchor });
  }

  // ── Layout edits ──────────────────────────────────────────────────────

  private patchEdge(id: string, patch: Partial<EdgeLayout>, save = true): void {
    const l = this.layout();
    this.local.set({ ...l, edges: { ...l.edges, [id]: { ...(l.edges[id] ?? {}), ...patch } } });
    if (save) this.scheduleSave();
  }

  private withStyle(style: ElementStyle | undefined, c: StyleChange): ElementStyle | undefined {
    const next: Record<string, unknown> = { ...(style ?? {}) };
    if (c.value === null || c.value === undefined || c.value === false) delete next[c.key];
    else next[c.key] = c.value;
    return Object.keys(next).length ? (next as ElementStyle) : undefined;
  }

  /** Applies a style change to every selected element (one save). */
  applyStyle(c: StyleChange): void {
    const l = structuredClone(this.layout());
    for (const s of this.selection()) {
      if (s.kind === 'node' && l.nodes[s.id]) l.nodes[s.id].style = this.withStyle(l.nodes[s.id].style, c);
      if (s.kind === 'zone' && l.zones[s.id]) l.zones[s.id].style = this.withStyle(l.zones[s.id].style, c);
      if (s.kind === 'edge') l.edges[s.id] = { ...(l.edges[s.id] ?? {}), style: this.withStyle(l.edges[s.id]?.style, c) };
    }
    this.local.set(l);
    this.scheduleSave();
  }

  resetStyle(): void {
    const l = structuredClone(this.layout());
    for (const s of this.selection()) {
      if (s.kind === 'node' && l.nodes[s.id]) delete l.nodes[s.id].style;
      if (s.kind === 'zone' && l.zones[s.id]) delete l.zones[s.id].style;
      if (s.kind === 'edge' && l.edges[s.id]) delete l.edges[s.id].style;
    }
    this.local.set(l);
    this.scheduleSave();
  }

  setShape(id: string, shape: NodeShape | null): void {
    const l = structuredClone(this.layout());
    if (!l.nodes[id]) return;
    if (shape) l.nodes[id].shape = shape;
    else delete l.nodes[id].shape;
    this.local.set(l);
    this.scheduleSave();
  }

  setShapeOfSelection(shape: NodeShape | null): void {
    for (const s of this.selection()) if (s.kind === 'node') this.setShape(s.id, shape);
  }

  setRouting(id: string, routing: 'curved' | 'orthogonal'): void {
    this.patchEdge(id, { routing: routing === 'curved' ? undefined : routing });
  }

  straighten(id: string): void {
    this.patchEdge(id, { curve: 0 });
  }

  private fitZone(id: string): void {
    const members = this.store.components().filter(c => c.zoneId === id).map(c => this.layout().nodes[c.id]);
    const r = fitAround(members);
    if (!r) return;
    const l = this.layout();
    this.local.set({ ...l, zones: { ...l.zones, [id]: { ...l.zones[id], ...r } } });
    this.scheduleSave();
  }

  // ── Commands ──────────────────────────────────────────────────────────

  /** Adds a component of the chosen type, at `at` or in the middle of the view. */
  async addComponent(at?: { x: number; y: number }): Promise<void> {
    const v = this.view(), { w, h } = this.size();
    const centre = at ?? { x: (w / 2 - v.panX) / v.zoom, y: (h / 2 - v.panY) / v.zoom };
    const n = this.store.components().length + 1;
    try {
      const c = await this.store.save('components', {
        name: `${label(this.newType())} ${n}`, type: this.newType(), internetFacing: false, assetIds: [],
      });
      const spot = at
        ? { x: at.x - NODE_W / 2, y: at.y - NODE_H / 2 }
        : freeSpot(this.boxes(), NODE_W, NODE_H, { x: centre.x - 200, y: centre.y - 150, w: 400, h: 300 });
      const l = this.layout();
      this.local.set({ ...l, nodes: { ...l.nodes, [c.id]: { ...spot, w: NODE_W, h: NODE_H } } });
      this.scheduleSave();
      await this.dropInZone(c.id);
      const fresh = this.store.components().find(x => x.id === c.id);
      if (fresh) { this.selection.set([{ kind: 'node', id: c.id }]); this.openDetail('details'); }
    } catch (err) {
      this.message.set(errorMessage(err));
    }
  }

  private async addZone(at: { x: number; y: number }): Promise<void> {
    try {
      const z = await this.store.save('zones', { name: `Zone ${this.store.zones().length + 1}`, kind: 'internal', trustLevel: 2 });
      const l = this.layout();
      this.local.set({ ...l, zones: { ...l.zones, [z.id]: { x: at.x, y: at.y, w: 260, h: 160 } } });
      this.scheduleSave();
      this.nav.go('zones', { select: z.id });
    } catch (err) {
      this.message.set(errorMessage(err));
    }
  }

  /** "Move to <zone>": the component goes into the region and the zone changes. */
  private async moveToZone(c: ArchComponent, zoneId: string | null): Promise<void> {
    if (zoneId) {
      const zr = this.layout().zones[zoneId];
      const taken = Object.entries(this.layout().nodes).filter(([id]) => id !== c.id).map(([, r]) => r);
      const spot = freeSpot(taken, NODE_W, NODE_H, zr);
      const l = this.layout();
      this.local.set({ ...l, nodes: { ...l.nodes, [c.id]: { ...l.nodes[c.id], ...spot } } });
      this.scheduleSave();
    }
    await this.setZone(c, zoneId);
  }

  private async reverse(f: Flow): Promise<void> {
    try {
      await this.store.save('flows', { ...flowInput(f), sourceId: f.targetId, targetId: f.sourceId }, f);
      const e = this.layout().edges[f.id] ?? {};
      this.patchEdge(f.id, { fromAnchor: e.toAnchor, toAnchor: e.fromAnchor, curve: e.curve ? -e.curve : e.curve });
    } catch (err) {
      this.message.set(errorMessage(err));
    }
  }

  openNode(c: ArchComponent): void {
    this.selection.set([{ kind: 'node', id: c.id }]);
    this.openDetail('details');
  }

  openFlow(f: Flow): void {
    this.selection.set([{ kind: 'edge', id: f.id }]);
    this.openDetail('details');
  }

  openZone(id: string): void {
    this.selection.set([{ kind: 'zone', id }]);
    this.openDetail('details');
  }

  /**
   * Opens the detail view of the single selected element on a tab: Details is
   * its form, Properties its style. With several elements selected there is
   * no single form, so the stand-alone Properties panel opens instead.
   */
  openDetail(tab: 'details' | 'properties'): void {
    this.closeDetails();
    const sel = this.selection();
    if (sel.length !== 1) {
      this.showProps.set(tab === 'properties' && sel.length > 1);
      return;
    }
    this.showProps.set(false);
    const [s] = sel;
    if (s.kind === 'node') {
      const c = this.store.components().find(x => x.id === s.id);
      if (c) { this.componentsEditor().drawerTab.set(tab); this.componentsEditor().edit(c); }
    } else if (s.kind === 'edge') {
      const f = this.store.flows().find(x => x.id === s.id);
      if (f) { this.flowsEditor().drawerTab.set(tab); this.flowsEditor().edit(f); }
    } else {
      const z = this.store.zones().find(x => x.id === s.id);
      if (z) { this.zonesEditor().drawerTab.set(tab); this.zonesEditor().edit(z); }
    }
  }

  private closeDetails(): void {
    this.componentsEditor().editor.close();
    this.flowsEditor().editor.close();
    this.zonesEditor().editor.close();
  }

  /** Opens the Details tab of the single selected element. */
  editSelection(): void {
    this.openDetail('details');
  }

  openThreats(id: string, ev?: Event): void {
    ev?.stopPropagation();
    this.nav.go('threats', { target: id });
  }

  async deleteSelection(): Promise<void> {
    const sel = this.selection();
    if (!sel.length) return;
    const text = sel.length > 1
      ? `Delete ${sel.length} elements?`
      : sel[0].kind === 'node'
        ? this.componentsEditor().deleteText(this.store.components().find(c => c.id === sel[0].id) ?? null)
        : sel[0].kind === 'edge' ? 'Delete this data flow?' : `Delete zone "${this.zoneLabel(sel[0].id)}"? Its components will have no zone.`;
    if (!confirm(text)) return;
    try {
      // Flows first: deleting a component deletes its flows on the server.
      for (const kind of ['edge', 'node', 'zone'] as const) {
        for (const s of sel.filter(x => x.kind === kind)) {
          const exists = kind === 'edge' ? this.store.flows().some(f => f.id === s.id)
            : kind === 'node' ? this.store.components().some(c => c.id === s.id) : true;
          if (exists) await this.store.remove(kind === 'edge' ? 'flows' : kind === 'node' ? 'components' : 'zones', s.id);
        }
      }
      this.selection.set([]);
    } catch (err) {
      this.message.set(errorMessage(err));
    }
  }

  // ── Context menu ──────────────────────────────────────────────────────

  openMenu(e: MouseEvent, kind: Menu['kind'], id?: string): void {
    e.preventDefault();
    e.stopPropagation();
    if (kind !== 'bg' && id && !this.isSelected(id)) this.selection.set([{ kind, id }]);
    if (kind === 'bg') this.selection.set([]);
    this.menu.set({ kind, id, sx: e.clientX, sy: e.clientY, at: this.toCanvas(e) });
  }

  closeMenu(): void {
    this.menu.set(null);
  }

  @HostListener('document:pointerdown', ['$event'])
  onDocumentDown(e: PointerEvent): void {
    if (this.menu() && !this.menuEl()?.nativeElement.contains(e.target as Node)) this.closeMenu();
  }

  onMenuKey(e: KeyboardEvent): void {
    const items = [...(this.menuEl()?.nativeElement.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'Escape') { e.preventDefault(); this.closeMenu(); this.svg().nativeElement.focus(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); items[(i + 1) % items.length]?.focus(); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); items[(i - 1 + items.length) % items.length]?.focus(); }
  }

  /** Runs a menu item; the menu closes first. */
  async run(action: string, arg?: string): Promise<void> {
    const m = this.menu();
    this.closeMenu();
    if (!m) return;
    const c = m.id ? this.store.components().find(x => x.id === m.id) : undefined;
    const f = m.id ? this.store.flows().find(x => x.id === m.id) : undefined;
    switch (action) {
      case 'edit': this.editSelection(); break;
      case 'properties': this.openDetail('properties'); break;
      case 'add-flow': if (c) this.flowsEditor().add(c.id); break;
      case 'shape': if (c) this.setShape(c.id, (arg as NodeShape) ?? null); break;
      case 'zone': if (c) await this.moveToZone(c, arg ?? null); break;
      case 'threats': if (m.id) this.openThreats(m.id); break;
      case 'delete': await this.deleteSelection(); break;
      case 'reverse': if (f) await this.reverse(f); break;
      case 'routing': if (m.id) this.setRouting(m.id, arg as 'curved' | 'orthogonal'); break;
      case 'straighten': if (m.id) this.straighten(m.id); break;
      case 'fit-zone': if (m.id) this.fitZone(m.id); break;
      case 'add-here': await this.addComponent(m.at); break;
      case 'zone-here': await this.addZone(m.at); break;
      case 'fit': this.fit(); break;
      case 'overlay': this.overlay.update(v => !v); break;
    }
  }

  menuEdge() {
    const m = this.menu();
    return m?.kind === 'edge' ? this.layout().edges[m.id!] ?? {} : null;
  }

  menuNodeShape(): NodeShape | null {
    const m = this.menu();
    return m?.kind === 'node' ? this.layout().nodes[m.id!]?.shape ?? null : null;
  }

  menuNode(): ArchComponent | undefined {
    const m = this.menu();
    return m?.kind === 'node' ? this.store.components().find(c => c.id === m.id) : undefined;
  }

  // ── Keyboard ──────────────────────────────────────────────────────────

  onNodeKey(e: KeyboardEvent, c: ArchComponent): void {
    if (e.key === 'Enter' || e.key === 'F2') {
      e.preventDefault();
      this.openNode(c);
    } else if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const r = (e.target as Element).getBoundingClientRect();
      this.openMenu(new MouseEvent('contextmenu', { clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 }), 'node', c.id);
    } else if (e.key.startsWith('Arrow')) {
      e.preventDefault();
      const step = e.shiftKey ? 80 : 8;
      const dx = e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0;
      const dy = e.key === 'ArrowUp' ? -step : e.key === 'ArrowDown' ? step : 0;
      const l = this.layout(), r = l.nodes[c.id];
      this.local.set({ ...l, nodes: { ...l.nodes, [c.id]: { ...r, x: r.x + dx, y: r.y + dy } } });
      this.scheduleSave();
    }
  }

  onKey(e: KeyboardEvent): void {
    const tag = (e.target as HTMLElement).tagName;
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;
    if (e.key === '!' || (e.shiftKey && e.code === 'Digit1')) { e.preventDefault(); this.fit(); }
    else if (e.key === '+' || e.key === '=') this.zoomBy(1.2);
    else if (e.key === '-') this.zoomBy(1 / 1.2);
    else if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); void this.deleteSelection(); }
    else if (e.key === 'Escape') { this.selection.set([]); this.closeMenu(); }
  }

  focusNode(c: ArchComponent): void {
    if (!this.isSelected(c.id)) this.selection.set([{ kind: 'node', id: c.id }]);
  }

  // ── Persistence ───────────────────────────────────────────────────────

  private scheduleSave(): void {
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => this.flushSave(), SAVE_DELAY_MS);
  }

  private flushSave(): void {
    if (!this.saveTimer) return;
    clearTimeout(this.saveTimer);
    this.saveTimer = null;
    void this.store.saveLayout(JSON.parse(JSON.stringify(this.layout())));
  }
}
