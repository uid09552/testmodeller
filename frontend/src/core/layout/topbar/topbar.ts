import {
  ChangeDetectionStrategy,
  Component,
  HostListener,
  output,
  signal,
} from '@angular/core';
import { BreadcrumbComponent } from './breadcrumb';
import { UserMenuComponent } from './user-menu';

@Component({
  selector: 'tm-topbar',
  imports: [BreadcrumbComponent, UserMenuComponent],
  templateUrl: './topbar.html',
  styleUrl: './topbar.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TopbarComponent {
  readonly menuClick = output<void>();

  readonly userMenuOpen = signal(false);

  toggleUserMenu(): void {
    this.userMenuOpen.update((v) => !v);
  }

  closeUserMenu(): void {
    this.userMenuOpen.set(false);
  }

  /** Close menu on outside click. */
  @HostListener('document:keydown.escape')
  onEscape(): void {
    this.closeUserMenu();
  }
}
