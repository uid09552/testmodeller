import { TestBed } from '@angular/core/testing';
import { CanvasComponent } from './canvas';
import { ModelEditorStore } from '../../state/model-editor.store';

describe('CanvasComponent state styling', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let el: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    el = fixture.nativeElement as HTMLElement;
    fixture.detectChanges();
  });

  const body = (id: string) =>
    el.querySelector(`[data-node-id="${id}"] .node-bg`) as SVGElement;
  const label = (id: string) =>
    el.querySelector(`[data-node-id="${id}"] .node-label`) as SVGTextElement;

  it('draws a dotted red outline with the chosen width on every shape', () => {
    for (const kind of ['regular', 'initial', 'decision'] as const) {
      const n = store.addNode(kind, 0, 0);
      store.setStyle([n.id], 'dash', 'dotted');
      store.setStyle([n.id], 'stroke', '#ef4444');
      store.setStyle([n.id], 'width', 3);
      fixture.detectChanges();
      const s = body(n.id).style;
      expect([s.stroke, s.strokeWidth, s.strokeDasharray, s.strokeLinecap])
        .toEqual(['rgb(239, 68, 68)', '3px', '0 8', 'round']);
    }
  });

  it('fills a state and returns to the default outline on "Default"', () => {
    const n = store.addNode('regular', 0, 0);
    store.setStyle([n.id], 'fill', '#123456');
    store.setStyle([n.id], 'stroke', '#ef4444');
    store.setStyle([n.id], 'stroke', null);
    fixture.detectChanges();
    expect(body(n.id).style.fill).toBe('rgb(18, 52, 86)');
    expect(body(n.id).style.stroke).toBe('');
    expect(body(n.id).classList).not.toContain('node-bg--outlined');
  });

  it('switches the label to light text on a dark fill unless a text colour is set', () => {
    const n = store.addNode('regular', 0, 0);
    store.setStyle([n.id], 'fill', '#1e293b');
    fixture.detectChanges();
    expect(label(n.id).classList).toContain('node-label--light');

    store.setStyle([n.id], 'text', '#fbbf24');
    fixture.detectChanges();
    expect(label(n.id).classList).not.toContain('node-label--light');
    expect(label(n.id).style.fill).toBe('rgb(251, 191, 36)');
  });

  it('draws a large bold italic label and grows the state to fit it', () => {
    const n = store.addNode('regular', 0, 0);
    store.updateNode(n.id, { label: 'Waiting for payment' });
    const before = store.nodeById(n.id)!.w;
    store.setStyle([n.id], 'size', 'l');
    store.setStyle([n.id], 'bold', true);
    store.setStyle([n.id], 'italic', true);
    fixture.detectChanges();
    const s = label(n.id).style;
    expect([s.fontSize, s.fontWeight, s.fontStyle]).toEqual(['16px', '700', 'italic']);
    expect(store.nodeById(n.id)!.w).toBeGreaterThan(before);
  });
});

describe('CanvasComponent transition styling', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let el: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    el = fixture.nativeElement as HTMLElement;
    fixture.detectChanges();
  });

  function chain(n: number) {
    const states = Array.from({ length: n + 1 }, (_, i) => store.addNode('regular', i * 300, 0));
    return states.slice(1).map((s, i) => store.addEdge(states[i].id, s.id));
  }
  const groups = () => [...el.querySelectorAll('.edge-group')] as SVGGElement[];
  const line = (i: number) => groups()[i].querySelector('.edge-line') as SVGPathElement;

  it('draws a dashed 3 px transition in its colour, with the label and guard styled', () => {
    const [e] = chain(1);
    store.updateEdge(e.id, { guard: 'ok' });
    store.setStyle([e.id], 'dash', 'dashed');
    store.setStyle([e.id], 'width', 3);
    store.setStyle([e.id], 'stroke', '#2563eb');
    store.setStyle([e.id], 'text', '#16a34a');
    store.setStyle([e.id], 'italic', true);
    store.setStyle([e.id], 'size', 'l');
    fixture.detectChanges();

    const s = line(0).style;
    expect([s.stroke, s.strokeWidth, s.strokeDasharray]).toEqual(['rgb(37, 99, 235)', '3px', '12 8']);
    const label = groups()[0].querySelector('.edge-label') as SVGTextElement;
    expect([label.style.fill, label.style.fontStyle, label.style.fontSize])
      .toEqual(['rgb(22, 163, 74)', 'italic', '14px']);
    const guard = groups()[0].querySelector('.edge-guard') as SVGTextElement;
    expect([guard.style.fill, guard.style.fontSize]).toEqual(['rgb(22, 163, 74)', '13px']);
  });

  it('keeps the shared arrowhead for plain transitions', () => {
    chain(1);
    fixture.detectChanges();
    expect(line(0).getAttribute('marker-end')).toBe('url(#arrow)');
  });

  it('draws arrowheads in the line colour and shares one marker per kind and colour', () => {
    const [a, b, c] = chain(3);
    store.setStyle([a.id, b.id], 'stroke', '#ef4444');
    store.setStyle([c.id], 'arrow', 'open');
    fixture.detectChanges();

    expect(line(0).getAttribute('marker-end')).toBe(line(1).getAttribute('marker-end'));
    const id = line(0).getAttribute('marker-end')!.slice(5, -1);
    const marker = el.querySelector(`marker#${id} polygon`) as SVGPolygonElement;
    expect(marker.style.fill).toBe('rgb(239, 68, 68)');

    const open = line(2).getAttribute('marker-end')!.slice(5, -1);
    expect(el.querySelector(`marker#${open} .edge-marker-open`)).not.toBeNull();
    expect(el.querySelectorAll('tm-canvas-edges marker, g[tmCanvasEdges] marker').length).toBe(2);
  });

  it('colours the arrowhead of a selected transition like the selection, keeping its kind', () => {
    const [e] = chain(1);
    store.setStyle([e.id], 'arrow', 'line');
    store.select(e.id, 'edge');
    fixture.detectChanges();
    const id = line(0).getAttribute('marker-end')!.slice(5, -1);
    expect(el.querySelector(`marker#${id} .edge-marker-line`)).not.toBeNull();
    expect(id).toContain('selected');
  });
});

/** Every style rule on the page, including the components' injected styles. */
function cssRules(): CSSStyleRule[] {
  return [...document.styleSheets].flatMap(sheet => {
    try { return [...sheet.cssRules]; } catch { return []; }
  }).filter((r): r is CSSStyleRule => 'selectorText' in r);
}

/** The text of every injected stylesheet. */
function styleText(): string {
  return [...document.querySelectorAll('style')].map(s => s.textContent ?? '').join('\n');
}

describe('CanvasComponent status cues over user styles', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let el: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    el = fixture.nativeElement as HTMLElement;
  });

  it('draws an uncovered dashed blue transition with the coverage cue, and its style again afterwards', () => {
    const a = store.addNode('initial', 0, 0);
    const b = store.addNode('regular', 300, 0);
    const e = store.addEdge(a.id, b.id);
    store.setStyle([e.id], 'dash', 'dashed');
    store.setStyle([e.id], 'stroke', '#2563eb');
    store.setTransitionCoverage({ covered: 0, total: 1, uncoveredTransitionIds: [e.id] });
    store.coverageView.set('transitions');
    fixture.detectChanges();

    const group = el.querySelector('.edge-group') as SVGGElement;
    const line = group.querySelector('.edge-line') as SVGPathElement;
    expect(group.classList).toContain('edge-group--uncovered');
    expect(group.querySelector('.edge-gap-marker text')?.textContent).toBe('0');
    // The cue's rule wins over the inline user style: it is !important.
    const rule = cssRules().find(r =>
      r.selectorText.includes('edge-group--uncovered') && r.selectorText.includes('edge-line'));
    expect(rule?.style.getPropertyPriority('stroke-dasharray')).toBe('important');
    // jsdom drops the priority of var() values, so read the stroke from the source.
    expect(styleText()).toMatch(/edge-group--uncovered[^{]*edge-line[^{]*\{\s*stroke: var\(--clr-warning\) !important/);

    store.coverageView.set('off');
    fixture.detectChanges();
    expect(group.classList).not.toContain('edge-group--uncovered');
    expect(group.querySelector('.edge-gap-marker')).toBeNull();
    expect([line.style.stroke, line.style.strokeDasharray]).toEqual(['rgb(37, 99, 235)', '8 5']);
  });

  it('keeps the selection outline on a styled state', () => {
    const n = store.addNode('regular', 0, 0);
    store.setStyle([n.id], 'dash', 'dotted');
    store.setStyle([n.id], 'stroke', '#ef4444');
    store.select(n.id, 'node');
    fixture.detectChanges();
    expect(el.querySelector(`[data-node-id="${n.id}"]`)!.classList).toContain('node-group--selected');
  });
});

describe('CanvasComponent state shapes', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<CanvasComponent>>;
  let el: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CanvasComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(CanvasComponent);
    store = TestBed.inject(ModelEditorStore);
    el = fixture.nativeElement as HTMLElement;
    fixture.detectChanges();
  });

  it('draws the new shapes as outline paths, styled like the others', () => {
    for (const shape of ['hexagon', 'parallelogram', 'cylinder', 'document'] as const) {
      const n = store.addNode('regular', 0, 0, shape);
      store.setStyle([n.id], 'fill', '#fef3c7');
      fixture.detectChanges();
      const body = el.querySelector(`[data-node-id="${n.id}"] .node-bg`) as SVGPathElement;
      expect(body.tagName.toLowerCase()).toBe('path');
      expect(body.getAttribute('d')).toMatch(/^M/);
      expect(body.style.fill).toBe('rgb(254, 243, 199)');
    }
  });

  it('makes a regular state a cylinder from the context menu, keeping its kind', () => {
    const n = store.addNode('regular', 0, 0);
    fixture.componentInstance.openContextMenu(new MouseEvent('contextmenu'), 'node', n.id);
    fixture.detectChanges();
    const items = [...el.querySelectorAll('.ctx-item')].map(b => b.textContent!.trim());
    expect(items).toEqual(expect.arrayContaining([
      'Make Circle', 'Make Rectangle', 'Make Diamond (decision)', 'Make Hexagon',
      'Make Parallelogram', 'Make Cylinder', 'Make Document',
    ]));
    fixture.componentInstance.runMenuAction('shape-cylinder');
    expect(store.nodeById(n.id)!.shape).toBe('cylinder');
    expect(store.nodeById(n.id)!.kind).toBe('regular');
  });
});
