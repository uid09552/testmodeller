import { CUSTOM_ELEMENTS_SCHEMA, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { ExplorerTreeComponent } from '../../../features/explorer/components/explorer-tree/explorer-tree';
import { ExplorerStore } from '../../../features/explorer/state/explorer.store';
import { SidenavComponent } from './sidenav';

describe('SidenavComponent', () => {
  function render(collapsed: boolean): HTMLElement {
    TestBed.configureTestingModule({
      imports: [SidenavComponent],
      providers: [provideRouter([]), { provide: ExplorerStore, useValue: { projects: signal([]) } }],
    });
    // The project tree has its own spec; here it is an inert element.
    TestBed.overrideComponent(SidenavComponent, {
      remove: { imports: [ExplorerTreeComponent] }, add: { schemas: [CUSTOM_ELEMENTS_SCHEMA] },
    });
    const f = TestBed.createComponent(SidenavComponent);
    f.componentRef.setInput('collapsed', collapsed);
    f.detectChanges();
    return f.nativeElement;
  }

  it('lists the cross-cutting views, with no Threat Modeling item (it lives in the project tree)', () => {
    const root = render(false);
    const labels = [...root.querySelectorAll('.sidenav__item-label')].map(e => e.textContent?.trim());
    expect(labels).toEqual(['Test Cases', 'Coverage', 'Traceability', 'AI Proposals', 'Settings']);
    expect(root.querySelector('a[href="/threat-modeling"]')).toBeNull();
  });

  it('keeps the accessible names when collapsed to icons', () => {
    const root = render(true);
    expect(root.querySelector('a[aria-label="Traceability"] .sidenav__item-label')).toBeNull();
    expect(root.querySelector('a[aria-label="Traceability"]')?.getAttribute('title')).toBe('Traceability');
  });
});
