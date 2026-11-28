import { TestBed } from '@angular/core/testing';
import { ApiError } from '../../../core/api/api-error';
import { ThreatModelApi } from '../../../core/api/threat-model-api';
import { ProjectThreats } from './project-threats';

describe('ProjectThreats', () => {
  function setup(ofProject: () => Promise<unknown>) {
    const api = {
      ofProject: vi.fn(ofProject),
      list: vi.fn(async () => [
        { id: 't1', displayId: 'THR-1', title: 'Account takeover', status: 'open' },
        { id: 't2', displayId: 'THR-2', title: 'Old', status: 'dismissed' },
      ]),
    };
    TestBed.configureTestingModule({ providers: [{ provide: ThreatModelApi, useValue: api }] });
    return { api, threats: TestBed.inject(ProjectThreats) };
  }

  it('labels the threats, keeps dismissed ones marked, and caches per project', async () => {
    const { api, threats } = setup(async () => ({ id: 'tm1' }));
    expect(await threats.options('p1')).toEqual([
      { id: 't1', label: 'THR-1 Account takeover' },
      { id: 't2', label: 'THR-2 Old (dismissed)' },
    ]);
    await threats.options('p1');
    expect(api.list).toHaveBeenCalledTimes(1);
    await threats.options('p1', true);
    expect(api.list).toHaveBeenCalledTimes(2);
  });

  it('offers nothing for a project without a threat model', async () => {
    const { api, threats } = setup(async () => { throw new ApiError('not found', 404); });
    expect(await threats.options('p1')).toEqual([]);
    expect(api.list).not.toHaveBeenCalled();
  });

  it('does not cache a failure', async () => {
    let fail = true;
    const { threats } = setup(async () => {
      if (fail) throw new ApiError('down', 503);
      return { id: 'tm1' };
    });
    await expect(threats.options('p1')).rejects.toThrow('down');
    fail = false;
    expect((await threats.options('p1')).length).toBe(2);
  });
});
