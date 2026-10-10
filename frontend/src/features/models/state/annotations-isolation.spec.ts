import { TestBed } from '@angular/core/testing';
import { ModelEditorStore } from './model-editor.store';
import { remoteFingerprint, toModelInput } from './model-mapping';
import { ModelTableComponent } from '../components/model-table/model-table';

/**
 * Notes and text boxes are not part of the model: whatever is computed from
 * the model must be the same with and without them (canvas-annotations).
 */
describe('annotations stay out of the model', () => {
  let store: ModelEditorStore;

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [ModelTableComponent], providers: [ModelEditorStore] });
    store = TestBed.inject(ModelEditorStore);
    const a = store.addNode('initial', 0, 0);
    const b = store.addNode('regular', 300, 0);
    store.addEdge(a.id, b.id);
  });

  function addNote() {
    const n = store.addAnnotation('note', 600, 0);
    store.setAnnotationText(n.id, 'Payment lookup is slow');
    return n;
  }

  it('sends the same graph, so validation and generation on the server see no difference', () => {
    const before = toModelInput(store.toPersisted('m'));
    addNote();
    const after = toModelInput(store.toPersisted('m'));
    expect({ ...after, layout: null }).toEqual({ ...before, layout: null });
    // …yet adding a note is a change the editor saves.
    expect(remoteFingerprint(store.toPersisted('m'))).not.toBe(remoteFingerprint({ ...store.toPersisted('m'), annotations: [] }));
  });

  it('adds no validation issue and no search hit', () => {
    const issues = store.issues();
    addNote();
    expect(store.issues()).toEqual(issues);
    expect(store.searchElements('payment')).toEqual([]);
  });

  it('is not listed in the table view', () => {
    addNote();
    const fixture = TestBed.createComponent(ModelTableComponent);
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).not.toContain('Payment lookup');
  });

  it('has no effect on coverage or on the state count', () => {
    const coverage = store.stateCoverage();
    addNote();
    expect(store.stateCoverage()).toEqual(coverage);
    expect(store.nodes().length).toBe(2);
  });
});
