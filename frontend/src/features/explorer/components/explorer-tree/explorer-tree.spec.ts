import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, Router } from '@angular/router';
import { ThreatModelApi } from '../../../../core/api/threat-model-api';
import { ThreatModelIndex } from '../../../threat-model/state/threat-model-index';
import { ExplorerStore } from '../../state/explorer.store';
import { ExplorerTreeComponent } from './explorer-tree';

const PROJECTS = [
  { id: 'p1', name: 'Shop', expanded: true, components: [{ id: 'c1', name: 'Payments', expanded: true, features: [] }] },
  { id: 'p2', name: 'Blog', expanded: true, components: [] },
];

describe('ExplorerTreeComponent: threat models', () => {
  let byProject: ReturnType<typeof signal<Record<string, { id: string; name: string } | null>>>;
  let index: { byProject: typeof byProject; of: (id: string) => { id: string; name: string } | null; create: ReturnType<typeof vi.fn>; remove: ReturnType<typeof vi.fn>; set: ReturnType<typeof vi.fn> };
  let router: Router;
  let root: HTMLElement;
  let fixture: ReturnType<typeof TestBed.createComponent<ExplorerTreeComponent>>;

  async function settle() {
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(async () => {
    byProject = signal({ p1: { id: 'tm1', name: 'Shop threat model' }, p2: null });
    index = {
      byProject,
      of: id => byProject()[id] ?? null,
      create: vi.fn(async (projectId: string, name: string) => {
        byProject.update(m => ({ ...m, [projectId]: { id: 'tm2', name } }));
        return { id: 'tm2', name };
      }),
      remove: vi.fn(async (projectId: string) => byProject.update(m => ({ ...m, [projectId]: null }))),
      set: vi.fn(),
    };
    await TestBed.configureTestingModule({
      imports: [ExplorerTreeComponent],
      providers: [
        provideRouter([]),
        { provide: ThreatModelApi, useValue: {} },
        { provide: ThreatModelIndex, useValue: index },
        {
          provide: ExplorerStore,
          useValue: {
            projects: signal(PROJECTS), error: signal(null), newItemPrompt: signal(null),
            isSelected: () => false, select: vi.fn(), toggleProject: vi.fn(),
          },
        },
      ],
    }).compileComponents();
    router = TestBed.inject(Router);
    vi.spyOn(router, 'navigate').mockResolvedValue(true);
    fixture = TestBed.createComponent(ExplorerTreeComponent);
    fixture.detectChanges();
    root = fixture.nativeElement;
  });

  afterEach(() => vi.restoreAllMocks());

  const items = () => [...root.querySelectorAll<HTMLElement>('.tree-item')];
  const threat = (name: string) => root.querySelector<HTMLElement>(`.tree-item--threat[aria-label="Threat model ${name}"]`);
  const menuItems = () => [...root.querySelectorAll('.ctx-menu [role="menuitem"]')].map(b => b.textContent?.trim());

  it('lists the threat model right under its project, with a shield icon', () => {
    const rows = items().map(r => r.getAttribute('aria-label'));
    expect(rows).toEqual(['Project Shop', 'Threat model Shop threat model', 'Component Payments', 'Project Blog']);
    expect(threat('Shop threat model')!.querySelector('svg.tree-item__icon--threat path')!.getAttribute('d')).toContain('M12 3l8 3');
  });

  it('shows no threat model item for a project without one', () => {
    expect(root.querySelectorAll('.tree-item--threat')).toHaveLength(1);
  });

  it('opens the threat model workspace on click', () => {
    threat('Shop threat model')!.click();
    expect(router.navigate).toHaveBeenCalledWith(['/threat-modeling', 'p1', 'overview']);
  });

  it('offers "New Threat Model" on a project without one, and creates and opens it', async () => {
    const blog = items().find(r => r.getAttribute('aria-label') === 'Project Blog')!;
    blog.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 50, clientY: 50 }));
    fixture.detectChanges();
    expect(menuItems()).toContain('New Threat Model');
    expect(menuItems()).not.toContain('Open Threat Model');
    [...root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('New Threat Model'))!
      .dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    await settle();
    expect(index.create).toHaveBeenCalledWith('p2', 'Blog threat model');
    expect(router.navigate).toHaveBeenCalledWith(['/threat-modeling', 'p2', 'overview']);
    expect(threat('Blog threat model')).not.toBeNull();
  });

  it('offers "Open Threat Model" instead on a project that has one', () => {
    items().find(r => r.getAttribute('aria-label') === 'Project Shop')!
      .dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 50, clientY: 50 }));
    fixture.detectChanges();
    expect(menuItems()).toContain('Open Threat Model');
    expect(menuItems()).not.toContain('New Threat Model');
  });

  it('deletes a threat model from its own menu, after confirming', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const open = () => {
      threat('Shop threat model')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 60, clientY: 60 }));
      fixture.detectChanges();
    };
    open();
    expect(menuItems()).toEqual(['Open Threat Model', 'Delete Threat Model']);
    const del = () => [...root.querySelectorAll<HTMLButtonElement>('.ctx-menu [role="menuitem"]')].find(b => b.textContent?.includes('Delete'))!
      .dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    del();
    await settle();
    expect(index.remove).not.toHaveBeenCalled();
    confirmSpy.mockReturnValue(true);
    open();
    del();
    await settle();
    expect(index.remove).toHaveBeenCalledWith('p1');
    expect(threat('Shop threat model')).toBeNull();
  });
});
