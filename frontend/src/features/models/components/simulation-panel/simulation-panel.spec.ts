import { TestBed } from '@angular/core/testing';
import { ApiError } from '../../../../core/api/api-error';
import { OrgApi, SimulationRequest, SimulationStep } from '../../../../core/api/org-api';
import { TestCaseInput } from '../../../../core/api/api.types';
import { ModelEditorStore } from '../../state/model-editor.store';
import { ModelPersistenceService } from '../../state/model-persistence';
import { SimulationPanelComponent } from './simulation-panel';

describe('SimulationPanelComponent', () => {
  let store: ModelEditorStore;
  let fixture: ReturnType<typeof TestBed.createComponent<SimulationPanelComponent>>;
  let root: HTMLElement;
  let requests: SimulationRequest[];
  let created: TestCaseInput[];
  let ids: { start: string; out: string; e1: string; e2: string };
  let answer: (req: SimulationRequest) => SimulationStep;

  beforeEach(async () => {
    requests = [];
    created = [];
    await TestBed.configureTestingModule({
      imports: [SimulationPanelComponent],
      providers: [
        ModelEditorStore,
        { provide: ModelPersistenceService, useValue: { adoptTest: () => undefined } },
        {
          provide: OrgApi,
          useValue: {
            simulate: async (_id: string, req: SimulationRequest) => { requests.push(req); return answer(req); },
            createTestCase: async (_f: string, input: TestCaseInput) => {
              created.push(input);
              return { ...input, id: 'tc-new', version: 1, featureId: 'f1',
                       assignments: input.assignments!.map(a => ({ ...a, testCaseId: 'tc-new' })) };
            },
          },
        },
      ],
    }).compileComponents();
    store = TestBed.inject(ModelEditorStore);
    const start = store.addNode('initial', 0, 0);
    store.updateNode(start.id, { label: 'Start' });
    const out = store.addNode('regular', 300, 0);
    store.updateNode(out.id, { label: 'Out' });
    const e1 = store.addEdge(start.id, out.id);
    store.updateEdge(e1.id, { label: 'open' });
    const e2 = store.addEdge(out.id, out.id);
    store.updateEdge(e2.id, { label: 'retry', guard: 'attempts < 1' });
    ids = { start: start.id, out: out.id, e1: e1.id, e2: e2.id };
    answer = req => {
      if (!req.stateId) {
        return { stateId: ids.start, env: { attempts: 0 }, final: false,
                 transitions: [{ transitionId: ids.e1, enabled: true }] };
      }
      return { stateId: ids.out, env: { attempts: 0 }, final: false,
               transitions: [{ transitionId: ids.e2, enabled: false, reason: 'guard `attempts < 1` is false' }] };
    };
    fixture = TestBed.createComponent(SimulationPanelComponent);
    fixture.componentRef.setInput('modelId', 'm1');
    fixture.componentRef.setInput('featureId', 'f1');
    fixture.detectChanges();
    root = fixture.nativeElement;
  });

  const click = async (text: string) => {
    const btn = [...root.querySelectorAll<HTMLButtonElement>('button')].find(b => b.textContent?.includes(text))!;
    btn.click();
    await fixture.whenStable();
    fixture.detectChanges();
  };

  it('starts, sends the graph as on screen, marks the canvas and lists options', async () => {
    store.updateEdge(ids.e1, { guard: 'edited but unsaved' });
    await click('Start simulation');
    expect(requests[0].graph.transitions!.find(t => t.id === ids.e1)!.guard).toBe('edited but unsaved');
    expect(requests[0].graph.layout).toBeUndefined();
    expect(root.querySelector('[role="status"]')!.textContent).toContain('Start');
    expect(root.textContent).toContain('attempts');
    expect(store.simulation()?.currentId).toBe(ids.start);
    expect(store.simulation()?.enabled.has(ids.e1)).toBe(true);
  });

  it('takes a transition, shows blocked ones with their reason and a stuck state, and steps back', async () => {
    await click('Start simulation');
    await click('open → Out');
    expect(requests[1]).toMatchObject({ stateId: ids.start, env: { attempts: 0 }, take: ids.e1 });
    expect(root.querySelector('.sim__blocked')!.textContent).toContain('blocked: guard `attempts < 1` is false');
    expect(root.textContent).toContain('stuck: no transition is enabled');
    expect(store.simulation()?.blocked.has(ids.e2)).toBe(true);

    await click('Step back');
    expect(store.simulation()?.currentId).toBe(ids.start);
  });

  it('saves the trail as a manual test case assigned in step order', async () => {
    await click('Start simulation');
    await click('open → Out');
    await click('Save as test case');
    expect(created[0]).toMatchObject({
      name: 'Simulation: Start → Out',
      steps: [{ action: 'open', expected: "State 'Out' is reached" }],
      assignments: [
        { modelId: 'm1', stateId: ids.start },
        { modelId: 'm1', transitionId: ids.e1, stepOrder: 1 },
        { modelId: 'm1', stateId: ids.out, stepOrder: 1 },
      ],
    });
    expect(store.nodeById(ids.start)!.tests.map(t => t.name)).toContain('Simulation: Start → Out');
    expect(root.textContent).toContain('Saved “Simulation: Start → Out”');
  });

  it('explains why a model cannot be simulated', async () => {
    answer = () => { throw new ApiError('the model has validation errors; fix them to simulate', 422,
      { title: 'x', status: 422, errors: [{ field: 'NO_INITIAL_STATE', message: 'NO_INITIAL_STATE: no initial state' }] } as never); };
    await click('Start simulation');
    expect(root.querySelector('[role="alert"]')!.textContent).toContain('NO_INITIAL_STATE: no initial state');
  });

  it('clears the canvas marks when closed', () => {
    store.simulation.set({ currentId: 'x', enabled: new Set(), blocked: new Set() });
    fixture.destroy();
    expect(store.simulation()).toBeNull();
  });
});
