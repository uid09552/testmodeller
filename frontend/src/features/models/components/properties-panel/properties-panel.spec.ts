import { TestBed } from '@angular/core/testing';
import { ModelEditorStore } from '../../state/model-editor.store';
import { PropertiesPanelComponent } from './properties-panel';

describe('PropertiesPanelComponent transition routing', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<PropertiesPanelComponent>>;
  let root: HTMLElement;
  let edgeId: string;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PropertiesPanelComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(PropertiesPanelComponent);
    store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 400, 0);
    edgeId = store.addEdge(a.id, b.id).id;
    store.select(edgeId, 'edge');
    fixture.detectChanges();
    root = fixture.nativeElement;
  });

  const button = (text: string) =>
    [...root.querySelectorAll<HTMLButtonElement>('button')].find(b => b.textContent?.trim() === text);

  it('sets the routing style', () => {
    const select = root.querySelector<HTMLSelectElement>('#edge-routing')!;
    select.value = 'orthogonal';
    select.dispatchEvent(new Event('change'));
    expect(store.edgeById(edgeId)!.routing).toBe('orthogonal');
  });

  it('straightens a bent transition', () => {
    store.setEdgeCurve(edgeId, 50);
    store.addWaypoint(edgeId, 0, { x: 200, y: 100 });
    fixture.detectChanges();
    button('Straighten')!.click();
    expect(store.edgeById(edgeId)!.curve).toBe(0);
    expect(store.edgeById(edgeId)!.waypoints).toBeUndefined();
  });

  it('lists bend points and removes one by keyboard-reachable button', () => {
    store.addWaypoint(edgeId, 0, { x: 100, y: 50 });
    store.addWaypoint(edgeId, 1, { x: 300, y: 50 });
    fixture.detectChanges();
    const remove = root.querySelector<HTMLButtonElement>('[aria-label="Remove bend point 1"]')!;
    remove.click();
    expect(store.edgeById(edgeId)!.waypoints).toEqual([{ x: 300, y: 50 }]);
  });

  it('resets a moved label', () => {
    expect(button('Reset label position')).toBeUndefined();
    store.setLabelOffsetLive(edgeId, 0, -40);
    fixture.detectChanges();
    button('Reset label position')!.click();
    expect(store.edgeById(edgeId)!.labelOffset).toBeUndefined();
  });
});

describe('PropertiesPanelComponent state shape', () => {
  it('offers all seven shapes and changes the shape, not the kind', async () => {
    await TestBed.configureTestingModule({
      imports: [PropertiesPanelComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(PropertiesPanelComponent);
    const store = TestBed.inject(ModelEditorStore);
    const n = store.addNode('decision', 0, 0);
    store.select(n.id, 'node');
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;

    const radios = [...root.querySelectorAll<HTMLButtonElement>('[aria-label="Shape"] [role="radio"]')];
    expect(radios.map(r => r.getAttribute('aria-label'))).toEqual([
      'Circle', 'Rectangle', 'Diamond (decision)', 'Hexagon', 'Parallelogram', 'Cylinder', 'Document',
    ]);
    radios[3].click();
    fixture.detectChanges();
    expect(store.nodeById(n.id)!.shape).toBe('hexagon');
    expect(store.nodeById(n.id)!.kind).toBe('decision');
    expect(radios[3].getAttribute('aria-checked')).toBe('true');
  });
});

describe('PropertiesPanelComponent styling', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<PropertiesPanelComponent>>;
  let root: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PropertiesPanelComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    fixture = TestBed.createComponent(PropertiesPanelComponent);
    store = TestBed.inject(ModelEditorStore);
    root = fixture.nativeElement;
  });

  const option = (name: string) => root.querySelector<HTMLButtonElement>(`button[aria-label="${name}"]`)!;
  const has = (name: string) => root.querySelector(`[aria-label="${name}"]`) !== null;

  it('styles a single state, replacing the old border colour row', () => {
    const n = store.addNode('regular', 0, 0);
    store.select(n.id, 'node');
    fixture.detectChanges();
    expect(root.textContent).not.toContain('Border colour');
    option('Dotted').click();
    option('Fill colour: Teal').click();
    expect(store.nodeById(n.id)!.style).toEqual({ dash: 'dotted', fill: '#14b8a6' });
  });

  it('styles a single transition, including its arrowhead', () => {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 300, 0);
    const e = store.addEdge(a.id, b.id);
    store.select(e.id, 'edge');
    fixture.detectChanges();
    option('Open').click();
    option('Width 3 px').click();
    expect(store.edgeById(e.id)!.style).toEqual({ arrow: 'open', width: 3 });
  });

  it('applies one change to every selected state and transition, as one undo step', () => {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 300, 0);
    const e = store.addEdge(a.id, b.id);
    store.selection.set([{ id: a.id, type: 'node' }, { id: b.id, type: 'node' }, { id: e.id, type: 'edge' }]);
    fixture.detectChanges();
    expect(has('Fill colour')).toBe(false);
    expect(has('Arrowhead')).toBe(false);

    option('Dashed').click();
    expect([store.nodeById(a.id)!.style, store.nodeById(b.id)!.style, store.edgeById(e.id)!.style])
      .toEqual([{ dash: 'dashed' }, { dash: 'dashed' }, { dash: 'dashed' }]);
    store.undo();
    expect(store.edgeById(e.id)!.style).toBeUndefined();
    expect(store.nodeById(a.id)!.style).toBeUndefined();
  });

  it('shows the mixed state for a multi-selection with different values', () => {
    const a = store.addNode('regular', 0, 0);
    const b = store.addNode('regular', 300, 0);
    store.setStyle([a.id], 'bold', true);
    store.selectNodes([a.id, b.id]);
    fixture.detectChanges();
    expect(option('Bold').getAttribute('aria-pressed')).toBe('mixed');
  });
});

describe('PropertiesPanelComponent annotations', () => {
  it('edits a note\'s text and fill, says it is not part of the model, and deletes it', async () => {
    await TestBed.configureTestingModule({
      imports: [PropertiesPanelComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(PropertiesPanelComponent);
    const store = TestBed.inject(ModelEditorStore);
    const n = store.addAnnotation('note', 0, 0);
    store.select(n.id, 'annotation');
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;

    expect(root.querySelector('.props__title')!.textContent).toBe('Note');
    expect(root.textContent).toContain('Not part of the model');
    const text = root.querySelector<HTMLTextAreaElement>('#note-text')!;
    text.value = 'Ask ops\nabout timeouts';
    text.dispatchEvent(new Event('change'));
    root.querySelector<HTMLButtonElement>('[aria-label="Fill colour: Blue"]')!.click();
    expect(store.annotationById(n.id)).toMatchObject({ text: 'Ask ops\nabout timeouts', style: { fill: '#4f6ef2' } });
    expect(root.querySelector('[aria-label="Arrowhead"]')).toBeNull();

    root.querySelector<HTMLButtonElement>('[aria-label="Delete note"]')!.click();
    expect(store.annotations()).toEqual([]);
  });

  it('offers only text style for a text box', async () => {
    await TestBed.configureTestingModule({
      imports: [PropertiesPanelComponent],
      providers: [ModelEditorStore],
    }).compileComponents();
    const fixture = TestBed.createComponent(PropertiesPanelComponent);
    const store = TestBed.inject(ModelEditorStore);
    const t = store.addAnnotation('text', 0, 0);
    store.select(t.id, 'annotation');
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;
    expect(root.querySelector('[aria-label="Text size"]')).not.toBeNull();
    expect(root.querySelector('[aria-label="Fill colour"]')).toBeNull();
    expect(root.querySelector('[aria-label="Line pattern"]')).toBeNull();
  });
});
