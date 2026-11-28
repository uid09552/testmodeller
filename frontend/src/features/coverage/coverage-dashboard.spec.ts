import { CUSTOM_ELEMENTS_SCHEMA, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import {
  IxEmptyState, IxKpi, IxMessageBar, IxPill, IxProgressIndicator, IxSelect, IxSelectItem,
  IxSpinner,
} from '@siemens/ix-angular/standalone';
import { ApiError } from '../../core/api/api-error';
import { OrgApi } from '../../core/api/org-api';
import { ExplorerProject, ExplorerStore } from '../explorer/state/explorer.store';
import { CoverageDashboardComponent } from './coverage-dashboard';

/** The iX wrappers start real web components; here they are inert elements. */
const INERT = {
  remove: {
    imports: [IxEmptyState, IxKpi, IxMessageBar, IxPill, IxProgressIndicator, IxSelect, IxSelectItem, IxSpinner],
  },
  add: { schemas: [CUSTOM_ELEMENTS_SCHEMA] },
};

const project = (id: string, models: string[]): ExplorerProject => ({
  id, name: `Project ${id}`, expanded: true,
  components: [{
    id: `${id}-c`, name: 'Checkout', expanded: true,
    features: [{
      id: `${id}-f`, name: 'Cart', description: '', expanded: true,
      models: models.map(m => ({ id: m, name: `Model ${m}`, status: 'draft' as const, states: 2, transitions: 1 })),
    }],
  }],
});

class NoIntersectionObserver {
  observe(): void { /* jsdom has no layout */ }
  unobserve(): void { /* nothing observed */ }
  disconnect(): void { /* nothing observed */ }
  takeRecords(): [] { return []; }
}

/** Lets the coverage requests resolve and renders the result. */
async function settle(f: { detectChanges(): void }): Promise<void> {
  for (let i = 0; i < 3; i++) await new Promise(r => setTimeout(r));
  f.detectChanges();
}

describe('CoverageDashboardComponent', () => {
  beforeAll(() => {
    (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver ??= NoIntersectionObserver;
  });

  async function render(projects: ExplorerProject[], coverage: (id: string) => Promise<unknown>) {
    const api = { modelCoverage: vi.fn(coverage) };
    TestBed.configureTestingModule({
      imports: [CoverageDashboardComponent],
      providers: [
        provideRouter([]),
        { provide: OrgApi, useValue: api },
        { provide: ExplorerStore, useValue: { projects: signal(projects) } },
      ],
    });
    TestBed.overrideComponent(CoverageDashboardComponent, INERT);
    const f = TestBed.createComponent(CoverageDashboardComponent);
    f.detectChanges();
    await f.whenStable();
    await settle(f);
    return { f, api };
  }

  it('loads each model of the first project and shows real coverage', async () => {
    const { f, api } = await render([project('p1', ['m1', 'm2']), project('p2', ['m3'])], async id =>
      id === 'm1'
        ? { states: { covered: 2, total: 2 }, transitions: { covered: 1, total: 1 } }
        : { states: { covered: 0, total: 2 }, transitions: { covered: 0, total: 1 } });

    expect(api.modelCoverage.mock.calls.map(c => c[0])).toEqual(['m1', 'm2']);
    const c = f.componentInstance;
    expect(c.totals()).toMatchObject({ models: 2, covered: 1, untested: 1, failed: 0 });
    expect(c.statePct()).toBe(50);
    const text = f.nativeElement.textContent as string;
    expect(text).toContain('Model m1');
    expect(text).toContain('Covered');
    expect(text).toContain('Not covered');
    expect(text).toContain('2 states, 1 transition uncovered');
    expect(text).not.toContain('Login Flow');
  });

  it('switches project', async () => {
    const { f, api } = await render([project('p1', ['m1']), project('p2', ['m3'])], async () =>
      ({ states: { covered: 1, total: 2 }, transitions: { covered: 0, total: 1 } }));
    f.componentInstance.setProject('p2');
    f.detectChanges();
    await settle(f);
    expect(api.modelCoverage).toHaveBeenLastCalledWith('m3');
    expect(f.componentInstance.rows().map(r => r.modelId)).toEqual(['m3']);
  });

  it('reports a model whose coverage failed and keeps the rest', async () => {
    const { f } = await render([project('p1', ['m1', 'm2'])], async id => {
      if (id === 'm2') throw new ApiError('internal error', 500);
      return { states: { covered: 1, total: 1 }, transitions: { covered: 1, total: 1 } };
    });
    f.detectChanges();
    expect(f.componentInstance.totals().failed).toBe(1);
    expect(f.nativeElement.textContent).toContain('could not be');
  });

  it('shows an empty state without projects', async () => {
    const { f, api } = await render([], async () => ({}));
    expect(api.modelCoverage).not.toHaveBeenCalled();
    expect(f.nativeElement.querySelector('ix-empty-state')).not.toBeNull();
  });
});
