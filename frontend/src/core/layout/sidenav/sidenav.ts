import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { ExplorerTreeComponent } from '../../../features/explorer/components/explorer-tree/explorer-tree';

export interface NavItem {
  label: string;
  icon: string;   // SVG path data (single path, 24×24 viewBox)
  route: string;
  exact?: boolean;
}

const NAV_ITEMS: NavItem[] = [
  // Projects and models are reached through the tree above these items, not
  // through a nav entry of their own.
  {
    label: 'Test Cases',
    icon: 'M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2M9 5a2 2 0 0 0 2 2h2a2 2 0 0 0 2-2M9 5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2m-6 9 2 2 4-4',
    route: '/test-cases',
  },
  {
    label: 'Coverage',
    icon: 'M9 19v-6a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h2a2 2 0 0 0 2-2zm0 0V9a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v10m-6 0a2 2 0 0 0 2 2h2a2 2 0 0 0 2-2m0 0V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-2a2 2 0 0 1-2-2z',
    route: '/coverage',
  },
  {
    label: 'Traceability',
    icon: 'M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71',
    route: '/traceability',
  },
  {
    label: 'AI Proposals',
    icon: 'M9.663 17h4.673M12 3v1m6.364 1.636-.707.707M21 12h-1M4 12H3m3.343-5.657-.707-.707m2.828 9.9a5 5 0 1 1 7.072 0l-.548.547A3.374 3.374 0 0 0 14 18.469V19a2 2 0 1 1-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z',
    route: '/proposals',
  },
];

const BOTTOM_ITEMS: NavItem[] = [
  {
    label: 'Settings',
    icon: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 0 0-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 0 0-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 0 0-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 0 0-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 0 0 1.066-2.573c-.94-1.543.826-3.31 2.37-2.37 1 .608 2.296.07 2.572-1.065zM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
    route: '/settings',
  },
];

@Component({
  selector: 'tm-sidenav',
  imports: [RouterLink, RouterLinkActive, ExplorerTreeComponent],
  templateUrl: './sidenav.html',
  styleUrl: './sidenav.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SidenavComponent {
  readonly collapsed = input(false);
  readonly toggleCollapse = output<void>();

  readonly navItems = NAV_ITEMS;
  readonly bottomItems = BOTTOM_ITEMS;

  /**
   * The tree is the nav's main content, so it is always shown — there is no
   * separate toggle. Collapsing the nav to icons is what hides it, because at
   * that width there is no room for a tree.
   */
  showTree(): boolean {
    return !this.collapsed();
  }
}
