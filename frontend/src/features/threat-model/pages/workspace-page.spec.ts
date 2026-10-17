import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { ActivatedRoute, convertToParamMap, ParamMap, provideRouter, Router } from '@angular/router';
import { BehaviorSubject } from 'rxjs';
import { ThreatModelApi } from '../../../core/api/threat-model-api';
import { ExplorerStore } from '../../explorer/state/explorer.store';
import { FakeThreatModelApi } from '../testing/fixtures';
import { ThreatModelIndex } from '../state/threat-model-index';
import { PROJECTS } from '../testing/setup';
import { WorkspacePageComponent } from './workspace-page';

describe('WorkspacePageComponent', () => {
  let api: FakeThreatModelApi;
  let params: BehaviorSubject<ParamMap>;
  let navigate: ReturnType<typeof vi.fn>;

  async function render(p: Record<string, string>) {
    api = new FakeThreatModelApi();
    params = new BehaviorSubject(convertToParamMap(p));
    TestBed.configureTestingModule({
      imports: [WorkspacePageComponent],
      providers: [
        provideRouter([]),
        { provide: ThreatModelApi, useValue: api },
        { provide: ThreatModelIndex, useValue: { set: () => undefined } },
        { provide: ExplorerStore, useValue: { projects: signal(PROJECTS) } },
        { provide: ActivatedRoute, useValue: { paramMap: params, queryParamMap: new BehaviorSubject(convertToParamMap({})) } },
      ],
    });
    navigate = vi.spyOn(TestBed.inject(Router), 'navigate').mockResolvedValue(true) as never;
    return api;
  }

  async function create() {
    const f = TestBed.createComponent(WorkspacePageComponent);
    f.detectChanges();
    for (let i = 0; i < 3; i++) { await f.whenStable(); f.detectChanges(); }
    return f;
  }

  it('opens the first project when none is in the URL', async () => {
    await render({});
    await create();
    expect(navigate).toHaveBeenCalledWith(['/threat-modeling', 'p1', 'overview'], { replaceUrl: true });
  });

  it('shows the project and tab named in the URL', async () => {
    await render({ projectId: 'p1', tab: 'zones' });
    const f = await create();
    const root: HTMLElement = f.nativeElement;
    const active = root.querySelector('[role="tab"][aria-selected="true"]');
    expect(active?.textContent?.trim()).toBe('Zones');
    expect(root.querySelector('tm-zones-tab')).not.toBeNull();
  });

  it('offers to create a threat model and creates none by itself', async () => {
    await render({ projectId: 'p1' });
    api.model = null;
    const f = await create();
    const root: HTMLElement = f.nativeElement;
    expect(root.textContent).toContain('This project has no threat model yet.');
    expect(api.writes('create')).toEqual([]);
    [...root.querySelectorAll('button')].find(b => b.textContent?.includes('Create threat model'))!.click();
    for (let i = 0; i < 3; i++) { await f.whenStable(); f.detectChanges(); }
    expect(api.writes('create')).toHaveLength(1);
    expect(root.querySelector('[role="tablist"]')).not.toBeNull();
  });

  it('moves focus along the tabs with the arrow keys and opens one with a click or Enter', async () => {
    await render({ projectId: 'p1', tab: 'overview' });
    const f = await create();
    const tabs = [...(f.nativeElement as HTMLElement).querySelectorAll<HTMLButtonElement>('[role="tab"]')];
    tabs[0].focus();
    tabs[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    f.detectChanges();
    expect(document.activeElement).toBe(tabs[1]);
    expect(tabs[1].getAttribute('tabindex')).toBe('0');
    tabs[1].click();
    expect(navigate).toHaveBeenCalledWith(['/threat-modeling', 'p1', 'diagram'], { queryParams: {} });
  });
});
