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
