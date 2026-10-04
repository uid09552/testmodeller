/**
 * Signals-based store for the model editor canvas.
 * Provided at model-editor-page level so each editor gets its own instance.
 */
import { computed, Injectable, signal } from '@angular/core';

export type StateKind = 'initial' | 'regular' | 'decision' | 'final';

/** UML shape drawn for a state. */
export type NodeShape = 'circle' | 'rect' | 'diamond';

/** Default shape per kind: start is a circle, steps are rectangles,
 *  decisions are diamonds, and the end state keeps its rectangle. */
export const SHAPE_FOR_KIND: Record<StateKind, NodeShape> = {
  initial:  'circle',
  regular:  'rect',
  decision: 'diamond',
  final:    'rect',
};

/** Default footprint per shape. */
export const SIZE_FOR_SHAPE: Record<NodeShape, { w: number; h: number }> = {
  circle:    { w: 88,  h: 88 },
  rect:      { w: 144, h: 48 },
  diamond:   { w: 172, h: 104 },
};

/** Fill presets offered in the quick bar; `null` means "use the kind default". */
export const NODE_COLORS: { name: string; value: string | null }[] = [
  { name: 'Default', value: null },
  { name: 'Blue',    value: '#4f6ef2' },
  { name: 'Teal',    value: '#14b8a6' },
  { name: 'Green',   value: '#22c55e' },
  { name: 'Amber',   value: '#f59e0b' },
  { name: 'Red',     value: '#ef4444' },
  { name: 'Violet',  value: '#8b5cf6' },
  { name: 'Slate',   value: '#64748b' },
];

/** True when white text reads better than dark text on `hex`. */
export function needsLightText(hex: string): boolean {
  const h = hex.replace('#', '');
  const n = h.length === 3 ? h.split('').map(c => c + c).join('') : h;
  const r = parseInt(n.slice(0, 2), 16);
  const g = parseInt(n.slice(2, 4), 16);
  const b = parseInt(n.slice(4, 6), 16);
  // Rec. 601 luma
  return (0.299 * r + 0.587 * g + 0.114 * b) < 150;
}
export type ModelStatus = 'draft' | 'review' | 'approved';

export type TestCategory = 'unit' | 'integration' | 'feature';
export type TestPolarity = 'positive' | 'negative';

export const TEST_CATEGORIES: TestCategory[] = ['unit', 'integration', 'feature'];
export const TEST_POLARITIES: TestPolarity[] = ['positive', 'negative'];

/**
 * A Gherkin test case attached to a specific state.
 * `given`/`when`/`then` may each hold several lines; extra lines render and
 * export as `And` steps.
 */
export interface StateTest {
  /** Internal key. */
  id: string;
  /**
   * Stable per-model sequence number. The displayed id is derived as
   * `<ModelName>_<seq>` so renaming the model re-labels every test
   * consistently while the number itself never changes or gets reused.
   */
  seq: number;
  name: string;
  category: TestCategory;
  polarity: TestPolarity;
  given: string;
  when: string;
  then: string;
  /** Optional link to the implementing code / automated test. */
  implementationUrl?: string;
  /** Optional link to the backlog item this covers. */
  backlogUrl?: string;
}

/**
 * Normalise a user-supplied link for rendering as an `href`.
 *
 * Returns null unless the URL parses and uses http(s). These values are typed
 * by users and rendered as clickable links, so schemes like `javascript:` must
 * never reach the DOM — Angular would sanitise most of it, but rejecting here
 * keeps the stored value and the rendered link honest.
 */
export function safeExternalUrl(raw: string | undefined | null): string | null {
  const v = raw?.trim();
  if (!v) return null;
  try {
    const u = new URL(v);
    return u.protocol === 'http:' || u.protocol === 'https:' ? u.href : null;
  } catch {
    return null;
  }
}

/** Short label for a link, e.g. "github.com/…/login.spec.ts". */
export function linkLabel(raw: string | undefined | null): string {
  const safe = safeExternalUrl(raw);
  if (!safe) return '';
  const u = new URL(safe);
  const tail = u.pathname.split('/').filter(Boolean).pop();
  return tail ? `${u.hostname}/…/${tail}` : u.hostname;
}

/** Model name -> id prefix, e.g. "Login Flow" -> "LoginFlow". */
export function testRefPrefix(modelName: string): string {
  const cleaned = modelName.replace(/[^\p{L}\p{N}]+/gu, ' ').trim();
  if (!cleaned) return 'Model';
  return cleaned.split(/\s+/).map(w => w[0].toUpperCase() + w.slice(1)).join('');
}

/** Render one Gherkin clause, with continuation lines as `And`. */
export function gherkinClause(keyword: string, body: string, indent = '    '): string[] {
  const lines = body.split('\n').map(l => l.trim()).filter(Boolean);
  return lines.map((l, i) => `${indent}${i === 0 ? keyword : 'And'} ${l}`);
}

/** Full Gherkin text for one test case. */
export function testToGherkin(test: StateTest, ref?: string): string {
  const tags = `  @${test.category} @${test.polarity}`
    + (ref ? ` @id:${ref}` : '');
  const body = [
    ...gherkinClause('Given', test.given),
    ...gherkinClause('When',  test.when),
    ...gherkinClause('Then',  test.then),
  ];
  return [tags, `  Scenario: ${test.name}`, ...body].join('\n');
}

export interface CanvasNode {
  id: string;
  label: string;
  kind: StateKind;
  x: number;
  y: number;
  w: number;
  h: number;
  description?: string;
  tests: StateTest[];
  shape: NodeShape;
  /** Fill override; when unset the kind's default styling applies. */
  color?: string | null;
}

/** The four connector dots on a state. */
export type Anchor = 'top' | 'right' | 'bottom' | 'left';

export const ANCHORS: Anchor[] = ['top', 'right', 'bottom', 'left'];

/**
 * Position of a connector dot.
 *
 * Shape-independent: a rectangle, an inscribed ellipse and a diamond all touch
 * their bounding box at exactly these four points, so one formula serves all.
 */
export function anchorPoint(n: CanvasNode, a: Anchor): { x: number; y: number } {
  switch (a) {
    case 'top':    return { x: n.x + n.w / 2, y: n.y };
    case 'right':  return { x: n.x + n.w,     y: n.y + n.h / 2 };
    case 'bottom': return { x: n.x + n.w / 2, y: n.y + n.h };
    case 'left':   return { x: n.x,           y: n.y + n.h / 2 };
  }
}

/** The connector dot nearest to a point, for picking a drop target. */
export function nearestAnchor(n: CanvasNode, px: number, py: number): Anchor {
  let best: Anchor = 'top';
  let bestD = Infinity;
  for (const a of ANCHORS) {
    const p = anchorPoint(n, a);
    const d = (p.x - px) ** 2 + (p.y - py) ** 2;
    if (d < bestD) { bestD = d; best = a; }
  }
  return best;
}

export interface CanvasEdge {
  id: string;
  fromId: string;
  toId: string;
  label: string;
  guard?: string;
  action?: string;
  curve: number; // perpendicular bezier offset (0 = straight)
  /** Connector dots this transition joins. Absent on edges stored before
   *  anchors existed, which fall back to centre-to-centre geometry. */
  fromAnchor?: Anchor;
  toAnchor?: Anchor;
}

/**
 * A visual grouping of states, painted behind everything as a tinted panel.
 *
 * Membership is spatial: a state belongs to the group when its centre lies
 * inside the rectangle. That is what makes the group resizable — dragging an
 * edge takes states in or out — and it means there is no member list that can
 * fall out of step with the geometry.
 */
export interface CanvasGroup {
  id: string;
  label: string;
  /** Base colour; the fill is this at `opacity`, the border at full strength. */
  color: string;
  /** Fill opacity, 0–1. Defaults to 0.1. */
  opacity: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export type ResizeHandle = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';

export const GROUP_MIN_W = 120;
export const GROUP_MIN_H = 100;

export const GROUP_PAD = 26;
export const GROUP_LABEL_H = 26;

export const GROUP_COLORS = ['#4f6ef2', '#14b8a6', '#22c55e', '#f59e0b', '#ef4444', '#8b5cf6', '#64748b'];

export type AlignMode =
  | 'left' | 'center-h' | 'right'
  | 'top'  | 'middle'   | 'bottom'
  | 'dist-h' | 'dist-v';

export type SelType = 'node' | 'edge' | 'group';
export interface Selection { id: string; type: SelType }

export interface ValidationIssue {
  code: string;
  severity: 'error' | 'warning';
  message: string;
  elementId?: string;
}

interface Snapshot { nodes: CanvasNode[]; edges: CanvasEdge[]; groups: CanvasGroup[] }

export const NODE_W = 144;
export const NODE_H = 48;

@Injectable()
export class ModelEditorStore {
  // ── Model meta ────────────────────────────────────────────────────────────
  readonly name           = signal('Untitled Model');
  readonly description    = signal('');
  readonly scenarioDesc   = signal('');
  readonly modelStatus    = signal<ModelStatus>('draft');
  readonly dirty          = signal(false);

  // ── Graph ─────────────────────────────────────────────────────────────────
  readonly nodes  = signal<CanvasNode[]>([]);
  readonly edges  = signal<CanvasEdge[]>([]);
  readonly groups = signal<CanvasGroup[]>([]);

  // ── Selection ─────────────────────────────────────────────────────────────
  /** Everything currently selected. */
  readonly selection = signal<Selection[]>([]);

  /** The single selection, or null when zero or several are selected.
   *  Existing call sites (properties panel, canvas) read this unchanged. */
  readonly selected = computed<Selection | null>(() => {
    const sel = this.selection();
    return sel.length === 1 ? sel[0] : null;
  });

  readonly selectionCount = computed(() => this.selection().length);
  readonly hasMultiSelection = computed(() => this.selection().length > 1);

  /** Selected states, in model order. */
  readonly selectedNodes = computed(() => {
    const ids = new Set(this.selection().filter(s => s.type === 'node').map(s => s.id));
    return this.nodes().filter(n => ids.has(n.id));
  });

  readonly selectedGroup = computed(() => {
    const s = this.selected();
    if (!s || s.type !== 'group') return null;
    return this.groups().find(g => g.id === s.id) ?? null;
  });

  isSelectedId(id: string): boolean {
    return this.selection().some(s => s.id === id);
  }

  readonly selectedNode = computed(() => {
    const s = this.selected();
    if (!s || s.type !== 'node') return null;
    return this.nodes().find(n => n.id === s.id) ?? null;
  });

  readonly selectedEdge = computed(() => {
    const s = this.selected();
    if (!s || s.type !== 'edge') return null;
    return this.edges().find(e => e.id === s.id) ?? null;
  });

  /** Monotonic test-id counter; never rolled back, so ids are never reused. */
  private readonly testSeq = signal(1);

  // ── History ───────────────────────────────────────────────────────────────
  private readonly undoStack = signal<Snapshot[]>([]);
  private readonly redoStack = signal<Snapshot[]>([]);
  readonly canUndo = computed(() => this.undoStack().length > 0);
  readonly canRedo = computed(() => this.redoStack().length > 0);

  // ── Validation ────────────────────────────────────────────────────────────
  readonly issues = computed<ValidationIssue[]>(() => {
    const nodes = this.nodes();
    const edges = this.edges();
    const list: ValidationIssue[] = [];

    const initials = nodes.filter(n => n.kind === 'initial');
    if (initials.length === 0) list.push({ code: 'NO_INITIAL_STATE', severity: 'error', message: 'Model must have exactly one initial state.' });
    if (initials.length > 1) list.push({ code: 'MULTIPLE_INITIAL_STATES', severity: 'error', message: 'Model has multiple initial states.', elementId: initials[1]?.id });

    const outgoing = new Set(edges.map(e => e.fromId));
    for (const n of nodes.filter(n => n.kind !== 'final')) {
      if (!outgoing.has(n.id)) list.push({ code: 'DEAD_END', severity: 'warning', message: `"${n.label}" has no outgoing transitions.`, elementId: n.id });
    }

    if (initials.length === 1) {
      const reachable = new Set<string>();
      const q = [initials[0].id];
      while (q.length) {
        const id = q.shift()!;
        if (reachable.has(id)) continue;
        reachable.add(id);
        edges.filter(e => e.fromId === id).forEach(e => q.push(e.toId));
      }
      for (const n of nodes.filter(n => !reachable.has(n.id))) {
        list.push({ code: 'UNREACHABLE_STATE', severity: 'warning', message: `"${n.label}" is unreachable.`, elementId: n.id });
      }
    }

    const nodeIds = new Set(nodes.map(n => n.id));
    for (const e of edges) {
      if (!nodeIds.has(e.fromId)) list.push({ code: 'UNKNOWN_STATE', severity: 'error', message: `Transition "${e.label}" has unknown source.`, elementId: e.id });
      if (!nodeIds.has(e.toId))   list.push({ code: 'UNKNOWN_STATE', severity: 'error', message: `Transition "${e.label}" has unknown target.`, elementId: e.id });
    }

    return list;
  });

  readonly errorCount   = computed(() => this.issues().filter(i => i.severity === 'error').length);
  readonly warningCount = computed(() => this.issues().filter(i => i.severity === 'warning').length);

  // ── Helpers ────────────────────────────────────────────────────────────────
  nodeById(id: string): CanvasNode | undefined { return this.nodes().find(n => n.id === id); }
  edgeById(id: string): CanvasEdge | undefined { return this.edges().find(e => e.id === id); }

  // ── History helpers ────────────────────────────────────────────────────────
  private snap(): Snapshot {
    return {
      nodes:  structuredClone(this.nodes()),
      edges:  structuredClone(this.edges()),
      groups: structuredClone(this.groups()),
    };
  }
  private pushUndo(): void {
    this.undoStack.update(s => [...s.slice(-49), this.snap()]);
    this.redoStack.set([]);
  }

  undo(): void {
    const stack = this.undoStack();
    if (!stack.length) return;
    const cur = this.snap();
    const prev = stack[stack.length - 1];
    this.undoStack.update(s => s.slice(0, -1));
    this.redoStack.update(s => [...s, cur]);
    this.nodes.set(prev.nodes);
    this.edges.set(prev.edges);
    this.groups.set(prev.groups ?? []);
    this.dirty.set(true);
  }

  redo(): void {
    const stack = this.redoStack();
    if (!stack.length) return;
    const cur = this.snap();
    const next = stack[stack.length - 1];
    this.redoStack.update(s => s.slice(0, -1));
    this.undoStack.update(s => [...s, cur]);
    this.nodes.set(next.nodes);
    this.edges.set(next.edges);
    this.groups.set(next.groups ?? []);
    this.dirty.set(true);
  }

  // ── Meta mutations ─────────────────────────────────────────────────────────
  setName(v: string):          void { this.name.set(v);         this.dirty.set(true); }
  setDescription(v: string):   void { this.description.set(v);  this.dirty.set(true); }
  setScenarioDesc(v: string):  void { this.scenarioDesc.set(v); this.dirty.set(true); }
  setStatus(v: ModelStatus):   void { this.modelStatus.set(v);  this.dirty.set(true); }

  // ── Node mutations ─────────────────────────────────────────────────────────
  addNode(kind: StateKind, x = 200, y = 200, shape?: NodeShape): CanvasNode {
    this.pushUndo();
    const s = shape ?? SHAPE_FOR_KIND[kind];
    const { w, h } = SIZE_FOR_SHAPE[s];
    const node: CanvasNode = {
      id: crypto.randomUUID(),
      label: kind === 'initial' ? 'Start'
           : kind === 'final'   ? 'End'
           : kind === 'decision' ? 'Decision?'
           : 'State',
      kind, x, y, w, h, tests: [], shape: s, color: null,
    };
    this.nodes.update(ns => [...ns, node]);
    this.dirty.set(true);
    return node;
  }

  /**
   * Patch a state. Deliberately does NOT push undo: it is called on every
   * keystroke from the label field. Discrete actions (shape, kind, colour,
   * reconnect) call `checkpoint()` first instead.
   */
  updateNode(id: string, changes: Partial<Omit<CanvasNode, 'id'>>): void {
    this.nodes.update(ns => ns.map(n => {
      if (n.id !== id) return n;
      const next = { ...n, ...changes };
      // Switching kind adopts that kind's shape (and its size) unless the
      // caller set a shape explicitly in the same change.
      if (changes.kind && changes.shape === undefined) {
        next.shape = SHAPE_FOR_KIND[changes.kind];
      }
      if (next.shape !== n.shape) {
        const { w, h } = SIZE_FOR_SHAPE[next.shape];
        next.w = w;
        next.h = h;
      }
      return next;
    }));
    this.dirty.set(true);
  }

  moveNode(id: string, x: number, y: number): void {
    this.nodes.update(ns => ns.map(n => n.id === id ? { ...n, x, y } : n));
    this.dirty.set(true);
  }

  removeNode(id: string): void {
    this.pushUndo();
    this.nodes.update(ns => ns.filter(n => n.id !== id));
    this.edges.update(es => es.filter(e => e.fromId !== id && e.toId !== id));
    this.selection.update(sel => sel.filter(s => s.id !== id));
    this.dirty.set(true);
  }

  // ── Edge mutations ─────────────────────────────────────────────────────────
  addEdge(fromId: string, toId: string, fromAnchor?: Anchor, toAnchor?: Anchor): CanvasEdge {
    this.pushUndo();
    // Only edges joining the *same two dots* need bending apart; different
    // dots already separate visually.
    const existing = this.edges().filter(e =>
      (e.fromId === fromId && e.toId === toId
        && e.fromAnchor === fromAnchor && e.toAnchor === toAnchor) ||
      (e.fromId === toId && e.toId === fromId
        && e.fromAnchor === toAnchor && e.toAnchor === fromAnchor),
    );
    const n = existing.length;
    const curve = n === 0 ? 0 : (n % 2 === 0 ? -46 * Math.ceil(n / 2) : 46 * Math.ceil(n / 2));
    const edge: CanvasEdge = {
      id: crypto.randomUUID(),
      fromId, toId,
      label: 'transition',
      curve,
      fromAnchor, toAnchor,
    };
    this.edges.update(es => [...es, edge]);
    this.dirty.set(true);
    return edge;
  }

  updateEdge(id: string, changes: Partial<Omit<CanvasEdge, 'id'>>): void {
    this.edges.update(es => es.map(e => e.id === id ? { ...e, ...changes } : e));
    this.dirty.set(true);
  }

  removeEdge(id: string): void {
    this.pushUndo();
    this.edges.update(es => es.filter(e => e.id !== id));
    this.selection.update(sel => sel.filter(s => s.id !== id));
    this.dirty.set(true);
  }

  // ── State test cases ──────────────────────────────────────────────────────
  addTest(nodeId: string, name = 'New test case'): StateTest | null {
    const node = this.nodeById(nodeId);
    if (!node) return null;
    this.pushUndo();
    // Defaults: feature-level, positive case.
    const seq = this.testSeq();
    this.testSeq.set(seq + 1);
    const test: StateTest = {
      id: crypto.randomUUID(),
      seq,
      name,
      category: 'feature',
      polarity: 'positive',
      given: `the system is in the "${node.label}" state`,
      when:  '',
      then:  '',
      implementationUrl: '',
      backlogUrl: '',
    };
    this.nodes.update(ns => ns.map(n =>
      n.id === nodeId ? { ...n, tests: [...n.tests, test] } : n,
    ));
    this.dirty.set(true);
    return test;
  }

  updateTest(nodeId: string, testId: string, changes: Partial<Omit<StateTest, 'id'>>): void {
    this.nodes.update(ns => ns.map(n =>
      n.id !== nodeId ? n : {
        ...n,
        tests: n.tests.map(t => t.id === testId ? { ...t, ...changes } : t),
      },
    ));
    this.dirty.set(true);
  }

  removeTest(nodeId: string, testId: string): void {
    this.pushUndo();
    this.nodes.update(ns => ns.map(n =>
      n.id !== nodeId ? n : { ...n, tests: n.tests.filter(t => t.id !== testId) },
    ));
    this.dirty.set(true);
  }

  readonly totalTests = computed(() =>
    this.nodes().reduce((sum, n) => sum + n.tests.length, 0));

  /** Per-category counts for a state, for the on-node indicators. */
  testCounts(node: CanvasNode): { category: TestCategory; count: number; negative: number }[] {
    return TEST_CATEGORIES
      .map(category => ({
        category,
        count: node.tests.filter(t => t.category === category).length,
        negative: node.tests.filter(t => t.category === category && t.polarity === 'negative').length,
      }))
      .filter(c => c.count > 0);
  }

  /** Displayed id for a test, e.g. "LoginFlow_3". */
  testRef(test: StateTest): string {
    return `${testRefPrefix(this.name())}_${test.seq}`;
  }

  /** Every test in the model, flattened with its owning state. */
  readonly allTests = computed(() =>
    this.nodes().flatMap(n =>
      n.tests.map(test => ({
        test,
        nodeId: n.id,
        nodeLabel: n.label,
        ref: `${testRefPrefix(this.name())}_${test.seq}`,
      })),
    ),
  );

  /** Whole-model Gherkin feature file. */
  toGherkin(): string {
    const scenarios = this.allTests()
      .map(({ test, ref }) => testToGherkin(test, ref))
      .join('\n\n');
    return `Feature: ${this.name()}\n\n${scenarios}\n`;
  }

  /**
   * Drop an in-progress transition on empty canvas: create a state there and
   * connect it, as one undo step.
   */
  addNodeWithEdge(
    fromId: string, x: number, y: number, fromAnchor?: Anchor,
  ): CanvasNode | null {
    const source = this.nodeById(fromId);
    if (!source) return null;
    this.pushUndo();
    const node: CanvasNode = {
      id: crypto.randomUUID(),
      label: 'State',
      kind: 'regular',
      x, y, w: NODE_W, h: NODE_H, tests: [],
      shape: 'rect', color: null,
    };
    // Point the new node's nearest dot back at the dot the drag came from.
    const origin = fromAnchor ? anchorPoint(source, fromAnchor)
                              : { x: source.x + source.w / 2, y: source.y + source.h / 2 };
    const edge: CanvasEdge = {
      id: crypto.randomUUID(),
      fromId, toId: node.id,
      label: 'transition',
      curve: 0,
      fromAnchor,
      toAnchor: nearestAnchor(node, origin.x, origin.y),
    };
    this.nodes.update(ns => [...ns, node]);
    this.edges.update(es => [...es, edge]);
    this.dirty.set(true);
    return node;
  }

  /** Record an undo point before a discrete, one-shot edit. */
  checkpoint(): void { this.pushUndo(); }

  // ── Groups ────────────────────────────────────────────────────────────────
  groupById(id: string): CanvasGroup | undefined {
    return this.groups().find(g => g.id === id);
  }

  /** States whose centre lies inside the group's rectangle. */
  groupMembers(g: CanvasGroup): CanvasNode[] {
    return this.nodes().filter(n => {
      const cx = n.x + n.w / 2;
      const cy = n.y + n.h / 2;
      return cx >= g.x && cx <= g.x + g.w && cy >= g.y && cy <= g.y + g.h;
    });
  }

  groupMemberCount(g: CanvasGroup): number { return this.groupMembers(g).length; }

  /** Shrink-wrap a group around the states currently inside it. */
  fitGroup(id: string): void {
    const g = this.groupById(id);
    if (!g) return;
    const members = this.groupMembers(g);
    if (members.length === 0) return;
    this.pushUndo();
    const x0 = Math.min(...members.map(n => n.x));
    const y0 = Math.min(...members.map(n => n.y));
    const x1 = Math.max(...members.map(n => n.x + n.w));
    const y1 = Math.max(...members.map(n => n.y + n.h));
    this.updateGroup(id, {
      x: x0 - GROUP_PAD,
      y: y0 - GROUP_PAD - GROUP_LABEL_H,
      w: Math.max((x1 - x0) + GROUP_PAD * 2, GROUP_MIN_W),
      h: Math.max((y1 - y0) + GROUP_PAD * 2 + GROUP_LABEL_H, GROUP_MIN_H),
    });
  }

  /** Resize a group, clamped to a minimum and never inverted. */
  resizeGroup(
    id: string, handle: ResizeHandle,
    orig: { x: number; y: number; w: number; h: number },
    dx: number, dy: number,
  ): void {
    let { x, y, w, h } = orig;

    if (handle.includes('w')) {
      // Dragging the left edge moves the origin, so width shrinks as x grows.
      const nw = Math.max(orig.w - dx, GROUP_MIN_W);
      x = orig.x + (orig.w - nw);
      w = nw;
    }
    if (handle.includes('e')) w = Math.max(orig.w + dx, GROUP_MIN_W);
    if (handle.includes('n')) {
      const nh = Math.max(orig.h - dy, GROUP_MIN_H);
      y = orig.y + (orig.h - nh);
      h = nh;
    }
    if (handle.includes('s')) h = Math.max(orig.h + dy, GROUP_MIN_H);

    this.updateGroup(id, { x, y, w, h });
  }

  /** Group the current multi-selection. */
  groupSelection(label = 'Group'): CanvasGroup | null {
    const members = this.selectedNodes();
    if (members.length < 2) return null;
    this.pushUndo();

    const x0 = Math.min(...members.map(n => n.x));
    const y0 = Math.min(...members.map(n => n.y));
    const x1 = Math.max(...members.map(n => n.x + n.w));
    const y1 = Math.max(...members.map(n => n.y + n.h));

    const used = this.groups().length;
    const group: CanvasGroup = {
      id: crypto.randomUUID(),
      label,
      color: GROUP_COLORS[used % GROUP_COLORS.length],
      opacity: 0.1,
      x: x0 - GROUP_PAD,
      y: y0 - GROUP_PAD - GROUP_LABEL_H,
      w: Math.max((x1 - x0) + GROUP_PAD * 2, GROUP_MIN_W),
      h: Math.max((y1 - y0) + GROUP_PAD * 2 + GROUP_LABEL_H, GROUP_MIN_H),
    };
    this.groups.update(gs => [...gs, group]);
    this.selection.set([{ id: group.id, type: 'group' }]);
    this.dirty.set(true);
    return group;
  }

  updateGroup(id: string, changes: Partial<Omit<CanvasGroup, 'id'>>): void {
    this.groups.update(gs => gs.map(g => g.id === id ? { ...g, ...changes } : g));
    this.dirty.set(true);
  }

  /** Remove the grouping; the states themselves are untouched. */
  ungroup(id: string): void {
    this.pushUndo();
    this.groups.update(gs => gs.filter(g => g.id !== id));
    this.selection.update(sel => sel.filter(s => s.id !== id));
    this.dirty.set(true);
  }

  /** Delete a group and every state inside it. */
  deleteGroupWithNodes(id: string): void {
    const g = this.groupById(id);
    if (!g) return;
    this.pushUndo();
    const ids = new Set(this.groupMembers(g).map(n => n.id));
    this.nodes.update(ns => ns.filter(n => !ids.has(n.id)));
    this.edges.update(es => es.filter(e => !ids.has(e.fromId) && !ids.has(e.toId)));
    this.groups.update(gs => gs.filter(x => x.id !== id));
    this.selection.set([]);
    this.dirty.set(true);
  }

  /** Move the panel and everything inside it. */
  moveGroup(id: string, dx: number, dy: number): void {
    const g = this.groupById(id);
    if (!g) return;
    const ids = new Set(this.groupMembers(g).map(n => n.id));
    this.nodes.update(ns => ns.map(n =>
      ids.has(n.id) ? { ...n, x: n.x + dx, y: n.y + dy } : n,
    ));
    this.groups.update(gs => gs.map(x =>
      x.id === id ? { ...x, x: x.x + dx, y: x.y + dy } : x,
    ));
    this.dirty.set(true);
  }

  // ── Mass operations on the selection ──────────────────────────────────────
  deleteSelection(): void {
    const sel = this.selection();
    if (sel.length === 0) return;
    this.pushUndo();
    const nodeIds = new Set(sel.filter(s => s.type === 'node').map(s => s.id));
    const edgeIds = new Set(sel.filter(s => s.type === 'edge').map(s => s.id));
    const groupIds = new Set(sel.filter(s => s.type === 'group').map(s => s.id));

    this.nodes.update(ns => ns.filter(n => !nodeIds.has(n.id)));
    this.edges.update(es => es.filter(e =>
      !edgeIds.has(e.id) && !nodeIds.has(e.fromId) && !nodeIds.has(e.toId)));
    this.groups.update(gs => gs.filter(g => !groupIds.has(g.id)));
    this.selection.set([]);
    this.dirty.set(true);
  }

  /** Apply a border colour to every selected state. */
  colorSelection(color: string | null): void {
    const ids = this.selectedNodes().map(n => n.id);
    if (ids.length === 0) return;
    this.pushUndo();
    const set = new Set(ids);
    this.nodes.update(ns => ns.map(n => set.has(n.id) ? { ...n, color } : n));
    this.dirty.set(true);
  }

  /** Align or distribute the selected states. */
  alignSelection(mode: AlignMode): void {
    const sel = this.selectedNodes();
    if (sel.length < 2) return;
    this.pushUndo();

    const ids = new Set(sel.map(n => n.id));
    const x0 = Math.min(...sel.map(n => n.x));
    const x1 = Math.max(...sel.map(n => n.x + n.w));
    const y0 = Math.min(...sel.map(n => n.y));
    const y1 = Math.max(...sel.map(n => n.y + n.h));
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;

    // Distribution needs even gaps between sorted edges, so precompute.
    const spread = new Map<string, { x?: number; y?: number }>();
    if (mode === 'dist-h' || mode === 'dist-v') {
      const horizontal = mode === 'dist-h';
      const sorted = [...sel].sort((a, b) => horizontal ? a.x - b.x : a.y - b.y);
      const total = horizontal ? x1 - x0 : y1 - y0;
      const used = sorted.reduce((sum, n) => sum + (horizontal ? n.w : n.h), 0);
      // When the states do not fit inside their current extent the even gap
      // comes out negative, which would overlap and hide them. Clamp to a
      // minimum and let the extent grow instead.
      const MIN_GAP = 16;
      const gap = Math.max((total - used) / (sorted.length - 1), MIN_GAP);
      let cursor = horizontal ? x0 : y0;
      for (const n of sorted) {
        spread.set(n.id, horizontal ? { x: cursor } : { y: cursor });
        cursor += (horizontal ? n.w : n.h) + gap;
      }
    }

    this.nodes.update(ns => ns.map(n => {
      if (!ids.has(n.id)) return n;
      switch (mode) {
        case 'left':     return { ...n, x: x0 };
        case 'right':    return { ...n, x: x1 - n.w };
        case 'center-h': return { ...n, x: cx - n.w / 2 };
        case 'top':      return { ...n, y: y0 };
        case 'bottom':   return { ...n, y: y1 - n.h };
        case 'middle':   return { ...n, y: cy - n.h / 2 };
        case 'dist-h':   return { ...n, x: spread.get(n.id)?.x ?? n.x };
        case 'dist-v':   return { ...n, y: spread.get(n.id)?.y ?? n.y };
      }
    }));
    this.dirty.set(true);
  }

  /** States whose bounding box intersects a marquee rectangle. */
  nodesInRect(x0: number, y0: number, x1: number, y1: number): string[] {
    const ax = Math.min(x0, x1), bx = Math.max(x0, x1);
    const ay = Math.min(y0, y1), by = Math.max(y0, y1);
    return this.nodes()
      .filter(n => n.x < bx && ax < n.x + n.w && n.y < by && ay < n.y + n.h)
      .map(n => n.id);
  }

  // ── Selection ─────────────────────────────────────────────────────────────
  select(id: string, type: SelType): void { this.selection.set([{ id, type }]); }
  deselect(): void { this.selection.set([]); }

  /** Replace the selection with these states (used by the marquee). */
  selectNodes(ids: string[]): void {
    this.selection.set(ids.map(id => ({ id, type: 'node' as const })));
  }

  /** Ctrl/Cmd-click: add or remove one element. */
  toggleSelect(id: string, type: SelType): void {
    this.selection.update(sel =>
      sel.some(s => s.id === id)
        ? sel.filter(s => s.id !== id)
        : [...sel, { id, type }],
    );
  }

  selectAll(): void {
    this.selection.set(this.nodes().map(n => ({ id: n.id, type: 'node' as const })));
  }
  clearDirty(): void { this.dirty.set(false); }

  /**
   * Tidy the diagram into layers: BFS depth from the initial state fixes the
   * column, and nodes keep their relative vertical order within a column so
   * the result still resembles what the user had. One undo step.
   */
  autoLayout(): void {
    const nodes = this.nodes();
    if (nodes.length === 0) return;
    this.pushUndo();

    const edges = this.edges();
    const byId = new Map(nodes.map(n => [n.id, n]));

    // Pick roots: the initial state, else anything with no incoming edge.
    const targets = new Set(edges.map(e => e.toId));
    let roots = nodes.filter(n => n.kind === 'initial').map(n => n.id);
    if (roots.length === 0) roots = nodes.filter(n => !targets.has(n.id)).map(n => n.id);
    if (roots.length === 0) roots = [nodes[0].id];

    // BFS depth; unreachable nodes land in a trailing column.
    const depth = new Map<string, number>();
    const queue: string[] = [...roots];
    roots.forEach(id => depth.set(id, 0));
    while (queue.length) {
      const id = queue.shift()!;
      const d = depth.get(id)!;
      for (const e of edges) {
        if (e.fromId !== id || e.toId === id) continue;
        if (!depth.has(e.toId)) {
          depth.set(e.toId, d + 1);
          queue.push(e.toId);
        }
      }
    }
    const maxDepth = Math.max(0, ...depth.values());
    for (const n of nodes) if (!depth.has(n.id)) depth.set(n.id, maxDepth + 1);

    // Group into columns, preserving each node's current vertical order.
    const columns = new Map<number, string[]>();
    for (const n of nodes) {
      const d = depth.get(n.id)!;
      const col = columns.get(d) ?? [];
      col.push(n.id);
      columns.set(d, col);
    }
    for (const col of columns.values()) {
      col.sort((a, b) => byId.get(a)!.y - byId.get(b)!.y);
    }

    // Column gap has to clear the 72px edge-label pill that sits at the
    // midpoint of a horizontal transition, plus its guard text underneath;
    // the row gap keeps stacked labels from colliding.
    const COL_GAP  = 170;
    const ROW_GAP  = 80;
    const ORIGIN_X = 80;
    const ORIGIN_Y = 80;

    // Vertical extent of the tallest column sets the centre line.
    const colHeights = [...columns.entries()].map(([d, ids]) => [
      d,
      ids.reduce((sum, id) => sum + byId.get(id)!.h, 0) + ROW_GAP * (ids.length - 1),
    ] as const);
    const tallest = Math.max(...colHeights.map(([, h]) => h));
    const centreY = ORIGIN_Y + tallest / 2;

    const pos = new Map<string, { x: number; y: number }>();
    let x = ORIGIN_X;
    for (const d of [...columns.keys()].sort((a, b) => a - b)) {
      const ids = columns.get(d)!;
      const colW = Math.max(...ids.map(id => byId.get(id)!.w));
      const colH = colHeights.find(([cd]) => cd === d)![1];
      let y = centreY - colH / 2;
      for (const id of ids) {
        const n = byId.get(id)!;
        pos.set(id, { x: x + (colW - n.w) / 2, y });   // centre within the column
        y += n.h + ROW_GAP;
      }
      x += colW + COL_GAP;
    }

    this.nodes.update(ns => ns.map(n => {
      const p = pos.get(n.id);
      return p ? { ...n, x: p.x, y: p.y } : n;
    }));
    this.dirty.set(true);
  }

  // ── Persistence bridge ────────────────────────────────────────────────────
  /** Replace all editor state with a stored model. */
  loadFrom(m: {
    name: string; description: string; scenarioDesc: string; status: ModelStatus;
    nodes: CanvasNode[]; edges: CanvasEdge[]; testSeq: number;
    groups?: CanvasGroup[];
  }): void {
    this.name.set(m.name);
    this.description.set(m.description);
    this.scenarioDesc.set(m.scenarioDesc);
    this.modelStatus.set(m.status);
    // Tolerate models stored before `tests` existed.
    this.nodes.set(m.nodes.map(n => ({
      ...n,
      tests: n.tests ?? [],
      shape: n.shape ?? SHAPE_FOR_KIND[n.kind] ?? 'rect',
      color: n.color ?? null,
    })));
    this.edges.set(m.edges);
    this.groups.set((m.groups ?? []).map(g => this.migrateGroup(g)));
    this.testSeq.set(m.testSeq || this.nextSeqFrom(m.nodes));
    this.undoStack.set([]);
    this.redoStack.set([]);
    this.selection.set([]);
    this.dirty.set(false);
  }

  /**
   * Groups were originally stored as a member list with derived bounds.
   * Give any such record an explicit rectangle around those members.
   */
  private migrateGroup(g: CanvasGroup & { nodeIds?: string[] }): CanvasGroup {
    if (typeof g.w === 'number' && typeof g.h === 'number') return g;
    const ids = g.nodeIds ?? [];
    const members = this.nodes().filter(n => ids.includes(n.id));
    if (members.length === 0) {
      return { ...g, x: g.x ?? 0, y: g.y ?? 0, w: GROUP_MIN_W, h: GROUP_MIN_H };
    }
    const x0 = Math.min(...members.map(n => n.x));
    const y0 = Math.min(...members.map(n => n.y));
    const x1 = Math.max(...members.map(n => n.x + n.w));
    const y1 = Math.max(...members.map(n => n.y + n.h));
    return {
      ...g,
      x: x0 - GROUP_PAD,
      y: y0 - GROUP_PAD - GROUP_LABEL_H,
      w: Math.max((x1 - x0) + GROUP_PAD * 2, GROUP_MIN_W),
      h: Math.max((y1 - y0) + GROUP_PAD * 2 + GROUP_LABEL_H, GROUP_MIN_H),
    };
  }

  private nextSeqFrom(nodes: CanvasNode[]): number {
    const max = nodes.flatMap(n => n.tests ?? []).reduce((m, t) => Math.max(m, t.seq ?? 0), 0);
    return max + 1;
  }

  /** Current state as a plain object for storage. */
  toPersisted(id: string) {
    return {
      id,
      name: this.name(),
      description: this.description(),
      scenarioDesc: this.scenarioDesc(),
      status: this.modelStatus(),
      nodes: this.nodes(),
      edges: this.edges(),
      groups: this.groups(),
      testSeq: this.testSeq(),
    };
  }

  /** Empty the editor (used when opening a brand-new model). */
  reset(): void {
    this.nodes.set([]);
    this.edges.set([]);
    this.groups.set([]);
    this.selection.set([]);
    this.description.set('');
    this.scenarioDesc.set('');
    this.modelStatus.set('draft');
    this.testSeq.set(1);
    this.undoStack.set([]);
    this.redoStack.set([]);
    this.dirty.set(false);
  }

  // ── Demo seed ─────────────────────────────────────────────────────────────
  loadDemo(): void {
    const s0 = this.addNode('initial', 80, 160);
    const s1 = this.addNode('regular', 280, 80);
    const s2 = this.addNode('regular', 280, 240);
    const s3 = this.addNode('final', 500, 160);
    s0.label = 'Start'; s1.label = 'Login'; s2.label = 'Guest'; s3.label = 'Home';
    this.nodes.update(ns => ns.map(n =>
      n.id === s0.id ? { ...n, label: 'Start' } :
      n.id === s1.id ? { ...n, label: 'Login' } :
      n.id === s2.id ? { ...n, label: 'Guest' } :
      n.id === s3.id ? { ...n, label: 'Home' } : n
    ));
    this.addEdge(s0.id, s1.id);
    this.addEdge(s0.id, s2.id);
    this.addEdge(s1.id, s3.id);
    this.addEdge(s2.id, s3.id);
    this.addEdge(s1.id, s2.id);
    this.dirty.set(false);
    this.testSeq.set(1);
    this.undoStack.set([]);
    this.redoStack.set([]);
  }
}
