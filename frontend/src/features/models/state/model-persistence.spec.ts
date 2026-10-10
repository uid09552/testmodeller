import { TestBed } from '@angular/core/testing';
import { ModelInput } from '../../../core/api/api.types';
import { OrgApi } from '../../../core/api/org-api';
import { ModelPersistenceService } from './model-persistence';

const audit = { version: 1, createdAt: '2026-01-01T00:00:00Z', updatedAt: '2026-01-01T00:00:00Z' };
const S = '11111111-1111-4111-8111-111111111111';

describe('ModelPersistenceService layout and variables', () => {
  let saves: ModelInput[];
  let service: ModelPersistenceService;

  beforeEach(() => {
    saves = [];
    const api = {
      getModel: async (id: string) => ({
        id, featureId: 'f1', name: 'Login', status: 'draft', ...audit,
        stateCount: 1, transitionCount: 0, transitions: [],
        variables: [{ name: 'attempts', type: 'integer', initial: 0 }],
        states: [{ id: S, name: 'Start', kind: 'initial', position: { x: 0, y: 0 } }],
        layout: { v: 1, states: { [S]: { color: '#ff0000' } } },
      }),
      modelTestCases: async () => [],
      getFeature: async () => ({ id: 'f1', componentId: 'c1', name: 'F', ...audit }),
      replaceModel: async (_id: string, version: number, input: ModelInput) => {
        saves.push(input);
        return { version: version + 1 };
      },
    };
    TestBed.configureTestingModule({
      providers: [ModelPersistenceService, { provide: OrgApi, useValue: api }],
    });
    service = TestBed.inject(ModelPersistenceService);
  });

  it('restores the stored layout on open', async () => {
    const m = await service.open('m1');
    expect(m?.nodes[0].style?.stroke).toBe('#ff0000');
  });

  it('saves a colour-only change, with the layout', async () => {
    const m = (await service.open('m1'))!;
    service.schedule({ ...m, nodes: [{ ...m.nodes[0], style: { stroke: '#00ff00' } }] });
    expect(service.state()).toBe('pending');
    await service.flush();

    expect(saves).toHaveLength(1);
    expect((saves[0].layout as { states: Record<string, { color: string }> }).states[S].color)
      .toBe('#00ff00');
  });

  it('sends the variables back after a move', async () => {
    const m = (await service.open('m1'))!;
    service.schedule({ ...m, nodes: [{ ...m.nodes[0], x: 50 }] });
    await service.flush();
    expect(saves[0].variables).toEqual([{ name: 'attempts', type: 'integer', initial: 0 }]);
  });

  it('sends nothing when nothing changed', async () => {
    const m = (await service.open('m1'))!;
    service.schedule(m);
    await service.flush();
    expect(saves).toEqual([]);
  });
});
