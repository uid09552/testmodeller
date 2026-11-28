import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, Router } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';
import { ExplorerStore } from '../../../features/explorer/state/explorer.store';
import { BreadcrumbComponent, buildCrumbs, modelCrumbs, modelIdOf } from './breadcrumb';

const PLACE = {
  projectId: 'p1', projectName: 'Webshop',
  componentId: 'c1', componentName: 'Checkout',
  featureId: 'f1', featureName: 'Payment',
  model: { id: 'm1', name: 'Card payment' },
};

@Component({ template: '' })
class Blank {}

describe('BreadcrumbComponent', () => {
  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        provideRouter([
          {
            path: 'models',
            data: { breadcrumb: 'Models' },
            children: [
              { path: ':modelId', component: Blank, data: { breadcrumb: 'Model Editor' } },
            ],
          },
          { path: 'explorer', component: Blank, data: { breadcrumb: 'Explorer' } },
        ]),
        { provide: ExplorerStore, useValue: { modelPaths: signal([PLACE]) } },
      ],
    });
  });

  it('builds crumbs without throwing across navigations', async () => {
    const harness = await RouterTestingHarness.create();
    const router = TestBed.inject(Router);

    // This is the sequence that used to throw "Cannot read properties of
    // undefined (reading 'data')" on every navigation.
    await harness.navigateByUrl('/explorer');
    await harness.navigateByUrl('/models/abc');
    await harness.navigateByUrl('/models/def');

    expect(buildCrumbs(router.routerState.snapshot.root)).toEqual([
      { label: 'Models', route: '/models' },
      { label: 'Model Editor', route: '/models/def' },
    ]);
  });

  it('links each crumb to its absolute path', async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl('/models/abc');

    const crumbs = buildCrumbs(TestBed.inject(Router).routerState.snapshot.root);
    // Not 'abc', which would resolve against the top bar's own route.
    expect(crumbs.at(-1)?.route).toBe('/models/abc');
  });

  it('copes with no route at all', () => {
    expect(buildCrumbs(null)).toEqual([]);
    expect(buildCrumbs(undefined)).toEqual([]);
  });

  it('renders once created, before and after navigating', async () => {
    const harness = await RouterTestingHarness.create();
    const fixture = TestBed.createComponent(BreadcrumbComponent);
    fixture.detectChanges();
    await harness.navigateByUrl('/explorer');
    fixture.detectChanges();

    expect(fixture.componentInstance.crumbs()).toEqual([
      { label: 'Explorer', route: '/explorer' },
    ]);
  });

  it('shows where an open model sits in the project tree', async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl('/models/m1');
    const crumbs = TestBed.createComponent(BreadcrumbComponent).componentInstance.crumbs();
    expect(crumbs.map(c => c.label)).toEqual(['Projects', 'Webshop', 'Checkout', 'Payment', 'Card payment']);
    expect(crumbs.at(-1)?.route).toBeUndefined();
  });

  it('falls back to the route crumbs for a model the tree does not know', async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl('/models/unknown');
    const crumbs = TestBed.createComponent(BreadcrumbComponent).componentInstance.crumbs();
    expect(crumbs.map(c => c.label)).toEqual(['Models', 'Model Editor']);
  });

  it('finds the model id anywhere in the chain', () => {
    expect(modelIdOf(null)).toBeNull();
    expect(modelCrumbs(PLACE)[0]).toEqual({ label: 'Projects', route: '/explorer' });
  });
});
