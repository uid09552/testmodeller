import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IxApplicationHeader,
  IxAvatar,
  IxDivider,
  IxDropdownItem,
  IxIconButton,
} from '@siemens/ix-angular/standalone';
import { iconCogwheel, iconLogOut, iconSearch, iconUserProfile } from '@siemens/ix-icons/icons';
import { SessionService } from '../../auth/session';
import { BreadcrumbComponent } from './breadcrumb';

/** The application header: product name, breadcrumb, global actions, user menu. */
@Component({
  selector: 'tm-topbar',
  imports: [IxApplicationHeader, IxAvatar, IxDivider, IxDropdownItem, IxIconButton, BreadcrumbComponent],
  templateUrl: './topbar.html',
  styleUrl: './topbar.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  // `ix-application` places this host in its header area.
  host: { slot: 'application-header' },
})
export class TopbarComponent {
  private readonly router = inject(Router);
  private readonly session = inject(SessionService);

  readonly icons = { search: iconSearch, profile: iconUserProfile, settings: iconCogwheel, signOut: iconLogOut };

  open(route: string): void {
    void this.router.navigateByUrl(route);
  }

  /** Ends the gateway session (FR-046). */
  signOut(): void {
    this.session.signOut();
  }
}
