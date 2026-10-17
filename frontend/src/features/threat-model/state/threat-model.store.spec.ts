import { TestBed } from '@angular/core/testing';
import { ThreatModelApi } from '../../../core/api/threat-model-api';
import { component, FakeThreatModelApi, zone } from '../testing/fixtures';
import { ThreatModelIndex } from './threat-model-index';
import { CONFLICT_MESSAGE, ThreatModelStore } from './threat-model.store';

describe('ThreatModelStore', () => {
  let api: FakeThreatModelApi;
  let store: ThreatModelStore;

  beforeEach(() => {
    api = new FakeThreatModelApi();
    TestBed.configureTestingModule({
      providers: [
        ThreatModelStore, { provide: ThreatModelApi, useValue: api },
        { provide: ThreatModelIndex, useValue: { set: () => undefined } },
      ],
    });
    store = TestBed.inject(ThreatModelStore);
  });

  it('loads the threat model and every collection', async () => {
    api.data.zones = [zone('z', 'Internet', 0)];
    api.data.components = [component('a', 'Web app')];
    await store.load('p1');
    expect(store.model()?.name).toBe('Shop');
    expect(store.zones().map(z => z.name)).toEqual(['Internet']);
    expect(store.components().map(c => c.name)).toEqual(['Web app']);
    expect(store.missing()).toBe(false);
  });

  it('reports a project without a threat model and creates nothing', async () => {
    api.model = null;
    await store.load('p1');
    expect(store.missing()).toBe(true);
    expect(store.error()).toBeNull();
    expect(api.writes('create')).toEqual([]);
  });

  it('creates the threat model on request', async () => {
    api.model = null;
    await store.load('p1');
    await store.create('Shop threat model');
    expect(api.writes('create')[0].args).toEqual(['p1', { name: 'Shop threat model' }]);
    expect(store.missing()).toBe(false);
    expect(store.model()?.name).toBe('Shop threat model');
  });

  it('reloads after a write so computed fields stay right', async () => {
    await store.load('p1');
    await store.save('components', {
      name: 'API', type: 'api', internetFacing: true, assetIds: [],
    });
    expect(store.components().map(c => c.name)).toEqual(['API']);
  });

  it('turns a version conflict into a reload message', async () => {
    await store.load('p1');
    api.failNext = { status: 412, message: 'version mismatch' };
    await expect(store.saveModel({ name: 'x' })).rejects.toBeTruthy();
    expect(store.error()).toBe(CONFLICT_MESSAGE);
  });

  it('saves the layout with its own version', async () => {
    await store.load('p1');
    await store.saveLayout({ v: 1 });
    expect(api.writes('saveLayout')[0].args).toEqual(['tm1', 0, { v: 1 }]);
    expect(store.model()?.layoutVersion).toBe(1);
  });
});
