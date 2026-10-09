import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  HostListener,
  inject,
  input,
  output,
  viewChild,
} from '@angular/core';
import { RouterLink } from '@angular/router';
import { SessionService } from '../../auth/session';

@Component({
  selector: 'tm-user-menu',
  imports: [RouterLink],
  templateUrl: './user-menu.html',
  styleUrl: './user-menu.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UserMenuComponent {
  readonly open = input(false);
  readonly dismiss = output<void>();

  private readonly session = inject(SessionService);
  private readonly panel = viewChild<ElementRef<HTMLElement>>('panel');

  /** Ends the gateway session (FR-046). */
  signOut(): void {
    this.dismiss.emit();
    this.session.signOut();
  }

  @HostListener('document:click', ['$event'])
  onDocClick(event: Event): void {
    if (!this.open()) return;
    const target = event.target as Node;
    const el = this.panel()?.nativeElement;
    if (el && !el.contains(target)) {
      this.dismiss.emit();
    }
  }
}
