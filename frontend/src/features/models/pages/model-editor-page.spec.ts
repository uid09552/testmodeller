import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';
import { ModelEditorPageComponent } from './model-editor-page';
import { OrgApi } from '../../../core/api/org-api';

const audit = { version: 1, createdAt: '2026-01-01T00:00:00Z', updatedAt: '2026-01-01T00:00:00Z' };

describe('ModelEditorPageComponent', () => {
  let harness: RouterTestingHarness;
  const requested: string[] = [];
  const coverageRequests: string[] = [];
  const staleRequests: string[] = [];

  beforeEach(async () => {
    requested.length = 0;
    coverageRequests.length = 0;
    staleRequests.length = 0;
    const api = {
      listProjects: async () => [],
      getModel: async (id: string) => {
        requested.push(id);
        return {
          id, featureId: 'f1', name: `Model ${id}`, status: 'draft', ...audit,
          stateCount: 1, transitionCount: 0, transitions: [],
          states: [{ id: 'a', modelId: id, name: 'Start', kind: 'initial', testCaseCount: 0 }],
        };
      },
      modelTestCases: async (id: string) => id !== 'm3' ? [] : [{
        id: 'tc1', version: 1, featureId: 'f1', name: 'Walk', steps: [], tags: [],
        assignments: [{ modelId: 'm3', stateId: 'a', testCaseId: 'tc1' }],
      }],
      staleTests: async (id: string) => {
        staleRequests.push(id);
        return [{ testCaseId: 'tc1', reasons: [{ code: 'STEP_UNASSIGNED', message: 'gone' }] }];
      },
      modelCoverage: async (id: string) => {
        coverageRequests.push(id);
        return {
          states: { covered: 0, total: 1 }, transitions: { covered: 1, total: 3 },
          uncoveredStateIds: ['a'], uncoveredTransitionIds: ['t2', 't3'],
        };
      },
      getFeature: async () => ({
        id: 'f1', componentId: 'c1', name: 'Login', scenarioDescription: 'As a user', ...audit,
      }),
    };
    TestBed.configureTestingModule({
      providers: [
        { provide: OrgApi, useValue: api },
        provideRouter([{ path: 'models/:modelId', component: ModelEditorPageComponent }]),
      ],
    });
    harness = await RouterTestingHarness.create();
  });

  const open = async (id: string) => {
    const page = await harness.navigateByUrl(`/models/${id}`, ModelEditorPageComponent);
    await new Promise(r => setTimeout(r));
    return page;
  };

  it('loads the model from the backend', async () => {
    const page = await open('m1');

    expect(page.store.name()).toBe('Model m1');
    expect(page.store.nodes().map(n => n.label)).toEqual(['Start']);
    expect(page.store.scenarioDesc()).toBe('As a user');
    expect(page.featureName()).toBe('Login');
  });

  it('loads the other model when the route parameter changes', async () => {
    await open('m1');
    const page = await open('m2');

    expect(page.store.name()).toBe('Model m2');
    expect(requested).toEqual(['m1', 'm2']);
  });

  it('keeps nothing in the browser', async () => {
    localStorage.clear();
    await open('m1');

    expect(Object.keys(localStorage).filter(k => /model|explorer/.test(k))).toEqual([]);
  });

  it('starts on the Test Cases tab', async () => {
    const page = await open('m1');

    expect(page.sideTab()).toBe('tests');
  });

  it('switches to Test Cases when the canvas asks to reveal them', async () => {
    const page = await open('m1');
    page.showTab('properties');

    page.revealTests();

    expect(page.sideTab()).toBe('tests');
  });

  it('fetches transition coverage of the saved model only when the overlay shows transitions', async () => {
    const page = await open('m1');
    page.store.coverageView.set('states');
    TestBed.tick();
    await new Promise(r => setTimeout(r));
    expect(coverageRequests).toEqual([]);

    page.store.coverageView.set('transitions');
    TestBed.tick();
    await new Promise(r => setTimeout(r));

    expect(coverageRequests).toEqual(['m1']);
    expect(page.store.transitionCoverage()).toEqual({
      covered: 1, total: 3, uncoveredTransitionIds: ['t2', 't3'],
    });
  });

  it('asks which generated tests are stale when the model opens', async () => {
    const page = await open('m1');
    await new Promise(r => setTimeout(r));

    expect(staleRequests).toEqual(['m1']);
    expect(page.store.staleReasons('tc1').map(r => r.code)).toEqual(['STEP_UNASSIGNED']);
  });

  it('opens with a test path highlighted from ?highlight=', async () => {
    const page = await harness.navigateByUrl('/models/m3?highlight=tc1', ModelEditorPageComponent);
    await new Promise(r => setTimeout(r));
    expect(page.store.highlight()?.name).toBe('Walk');
    expect(page.store.pathSteps('a')).toEqual([]);
  });

  it('presents read-only from ?view=present and returns with Esc', async () => {
    const page = await harness.navigateByUrl('/models/m1?view=present', ModelEditorPageComponent);
    await new Promise(r => setTimeout(r));
    harness.detectChanges();
    const root = harness.routeNativeElement!;
    expect(page.mode()).toBe('present');
    expect(root.querySelector('.editor-side')).toBeNull();
    expect(root.querySelector('tm-canvas')!.classList).toContain('readonly');

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await harness.fixture.whenStable();
    harness.detectChanges();
    expect(page.mode()).toBe('edit');
    expect(root.querySelector('.editor-side')).not.toBeNull();
  });

  it('shows the table view for ?view=table', async () => {
    await harness.navigateByUrl('/models/m1?view=table', ModelEditorPageComponent);
    await new Promise(r => setTimeout(r));
    harness.detectChanges();
    const root = harness.routeNativeElement!;
    expect(root.querySelector('tm-model-table')).not.toBeNull();
    expect(root.querySelector('tm-canvas')).toBeNull();
    expect(root.querySelector('.editor-side')).not.toBeNull();
  });
});
