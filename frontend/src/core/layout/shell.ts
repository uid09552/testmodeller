import { ChangeDetectionStrategy, Component, computed, signal } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { SidenavComponent } from './sidenav/sidenav';
import { TopbarComponent } from './topbar/topbar';

/** Root shell — holds sidenav + topbar + main outlet. */
@Component({
  selector: 'tm-shell',
  imports: [RouterOutlet, SidenavComponent, TopbarComponent],
  templateUrl: './shell.html',
  styleUrl: './shell.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ShellComponent {
  /** Whether the left nav is fully expanded or icon-only. */
  readonly collapsed = signal(false);

  readonly shellClass = computed(() => (this.collapsed() ? 'shell--collapsed' : 'shell--expanded'));

  toggleNav(): void {
    this.collapsed.update((v) => !v);
  }
}
