import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { IxMenu, IxMenuItem } from '@siemens/ix-angular/standalone';
import {
  iconAi,
  iconBarchart,
  iconCogwheel,
  iconDocumentSuccess,
  iconLink,
} from '@siemens/ix-icons/icons';
import { ExplorerTreeComponent } from '../../../features/explorer/components/explorer-tree/explorer-tree';

export interface NavItem {
  label: string;
  /** An iX icon (from `@siemens/ix-icons/icons`). */
  icon: string;
  route: string;
  exact?: boolean;
}

// Projects and models are reached through the tree above these items, not
// through a nav entry of their own (ADR 0004).
const NAV_ITEMS: NavItem[] = [
  { label: 'Test Cases', icon: iconDocumentSuccess, route: '/test-cases' },
  { label: 'Coverage', icon: iconBarchart, route: '/coverage' },
  { label: 'Traceability', icon: iconLink, route: '/traceability' },
  { label: 'AI Proposals', icon: iconAi, route: '/proposals' },
];

const BOTTOM_ITEMS: NavItem[] = [{ label: 'Settings', icon: iconCogwheel, route: '/settings' }];

/** The left navigation: an `ix-menu` whose main content is the project tree. */
@Component({
  selector: 'tm-sidenav',
  imports: [RouterLink, RouterLinkActive, IxMenu, IxMenuItem, ExplorerTreeComponent],
  templateUrl: './sidenav.html',
  styleUrl: './sidenav.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  // `ix-application` places this host in its menu area.
  host: { slot: 'menu' },
})
export class SidenavComponent {
  readonly navItems = NAV_ITEMS;
  readonly bottomItems = BOTTOM_ITEMS;

  /** Mirrors the menu's own expanded state. */
  readonly expanded = signal(true);

  /**
   * The tree is the nav's main content, so it is always shown — there is no
   * separate toggle. Collapsing the menu to icons is what hides it, because at
   * that width there is no room for a tree.
   */
  showTree(): boolean {
    return this.expanded();
  }

  onExpandChange(event: Event): void {
    this.expanded.set(!!(event as CustomEvent<boolean>).detail);
  }
}
