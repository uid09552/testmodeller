import { CUSTOM_ELEMENTS_SCHEMA, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { ExplorerTreeComponent } from '../../../features/explorer/components/explorer-tree/explorer-tree';
import { ExplorerStore } from '../../../features/explorer/state/explorer.store';
import { SidenavComponent } from './sidenav';

describe('SidenavComponent', () => {
  function render() {
    TestBed.configureTestingModule({
      imports: [SidenavComponent],
      providers: [provideRouter([]), { provide: ExplorerStore, useValue: { projects: signal([]) } }],
    });
    // The project tree has its own spec; here it is an inert element.
    TestBed.overrideComponent(SidenavComponent, {
      remove: { imports: [ExplorerTreeComponent] }, add: { schemas: [CUSTOM_ELEMENTS_SCHEMA] },
    });
    const f = TestBed.createComponent(SidenavComponent);
    f.detectChanges();
    return f;
  }

  const items = (root: HTMLElement) => [...root.querySelectorAll('ix-menu-item')] as HTMLElement[];

  it('lists the cross-cutting views, with no Threat Modeling item (it lives in the project tree)', () => {
    const root: HTMLElement = render().nativeElement;
    expect(items(root).map(e => e.getAttribute('aria-label'))).toEqual(
      ['Test Cases', 'Coverage', 'Traceability', 'AI Proposals', 'Settings']);
    expect(items(root).at(-1)?.getAttribute('slot')).toBe('bottom');
    expect(root.querySelector('[aria-label="Threat Modeling"]')).toBeNull();
  });

  it('sits in the application menu slot', () => {
    expect((render().nativeElement as HTMLElement).getAttribute('slot')).toBe('menu');
  });

  it('shows the project tree only while the menu is expanded, and keeps the accessible names', () => {
    const f = render();
    const root: HTMLElement = f.nativeElement;
    expect(root.querySelector('tm-explorer-tree')).not.toBeNull();

    root.querySelector('ix-menu')!.dispatchEvent(new CustomEvent('expandChange', { detail: false }));
    f.detectChanges();

    expect(root.querySelector('tm-explorer-tree')).toBeNull();
    expect(root.querySelector('ix-menu-item[aria-label="Traceability"]')).not.toBeNull();
  });
});
