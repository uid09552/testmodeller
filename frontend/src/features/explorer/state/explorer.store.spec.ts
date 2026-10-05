import { TestBed } from '@angular/core/testing';
import { OrgApi } from '../../../core/api/org-api';
import { ExplorerStore } from './explorer.store';

const audit = { version: 1, createdAt: '', updatedAt: '' };

function fakeApi() {
  const calls: string[] = [];
  const api = {
    listProjects: async () => [{ id: 'p1', name: 'P', ...audit }],
    listComponents: async () => [{ id: 'c1', projectId: 'p1', name: 'C', ...audit }],
    listFeatures: async () => [{ id: 'f1', componentId: 'c1', name: 'F', ...audit }],
    listModels: async () => [{
      id: 'm1', featureId: 'f1', name: 'M', status: 'ready',
      stateCount: 3, transitionCount: 2, ...audit,
    }],
    renameProject: async () => { calls.push('rename project'); return {}; },
    deleteModel: async () => { calls.push('delete model'); },
    createProject: async (name: string) => ({ id: 'p2', name, ...audit }),
  };
  return { api, calls };
}

async function setup() {
  const { api, calls } = fakeApi();
  TestBed.configureTestingModule({ providers: [{ provide: OrgApi, useValue: api }] });
  const store = TestBed.inject(ExplorerStore);
  await store.refresh();
  return { store, calls };
}

describe('ExplorerStore', () => {
  it('reads the tree from the backend, with model sizes and status', async () => {
    const { store } = await setup();

    const model = store.projects()[0].components[0].features[0].models[0];
    expect(model).toEqual({
      id: 'm1', name: 'M', status: 'approved', states: 3, transitions: 2,
    });
  });

  it('does not touch browser storage', async () => {
    localStorage.clear();
    await setup();

    expect(localStorage.length).toBe(0);
  });

  it('stays empty when everything is deleted, instead of seeding demo data', async () => {
    const { api } = fakeApi();
    api.listProjects = async () => [];
    TestBed.configureTestingModule({ providers: [{ provide: OrgApi, useValue: api }] });
    const store = TestBed.inject(ExplorerStore);
    await store.refresh();

    expect(store.projects()).toEqual([]);
  });

  it('renames in the backend before showing the new name', async () => {
    const { store, calls } = await setup();

    await store.renameProject('p1', 'Renamed');

    expect(calls).toEqual(['rename project']);
    expect(store.selectedProject()).toBeNull();
    expect(store.projects()[0].name).toBe('Renamed');
  });

  it('adds what the backend created, under the backend id', async () => {
    const { store } = await setup();

    const created = await store.addProject('New');

    expect(created?.id).toBe('p2');
    expect(store.projects().map(p => p.id)).toEqual(['p1', 'p2']);
  });

  it('resolves to null once the selected model is deleted', async () => {
    const { store, calls } = await setup();
    store.select({
      kind: 'model', projectId: 'p1', componentId: 'c1', featureId: 'f1', modelId: 'm1',
    });
    expect(store.selectedModel()?.id).toBe('m1');

    await store.deleteModel('p1', 'c1', 'f1', 'm1');

    expect(calls).toEqual(['delete model']);
    expect(store.selectedModel()).toBeNull();
  });

  it('highlights only the row at the selected level', async () => {
    const { store } = await setup();
    store.select({
      kind: 'model', projectId: 'p1', componentId: 'c1', featureId: 'f1', modelId: 'm1',
    });

    expect(store.isSelected('m1')).toBe(true);
    expect(store.isSelected('f1')).toBe(false);
  });

  it('shares the new-item dialog request so other screens can open it', async () => {
    const { store } = await setup();

    store.promptNew({ kind: 'component', projectId: 'p1' });
    expect(store.newItemPrompt()?.kind).toBe('component');
    store.closePrompt();
    expect(store.newItemPrompt()).toBeNull();
  });
});
