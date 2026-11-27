import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { IxApplication } from '@siemens/ix-angular/standalone';
import { SidenavComponent } from './sidenav/sidenav';
import { TopbarComponent } from './topbar/topbar';

/** Root shell: the iX application frame with header, menu and the routed page. */
@Component({
  selector: 'tm-shell',
  imports: [RouterOutlet, IxApplication, SidenavComponent, TopbarComponent],
  templateUrl: './shell.html',
  styleUrl: './shell.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ShellComponent {}
