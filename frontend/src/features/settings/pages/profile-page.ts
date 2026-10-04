import { ChangeDetectionStrategy, Component, computed, signal } from '@angular/core';

@Component({
  selector: 'tm-profile-page',
  templateUrl: './profile-page.html',
  styleUrl: './profile-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProfilePageComponent {
  readonly name  = signal('Max');
  readonly email = signal('max.rgbg@gmail.com');
  readonly role  = signal('Test Engineer');
  readonly saved = signal(false);

  readonly initials = computed(() =>
    this.name().trim().split(/\s+/).map(p => p[0] ?? '').join('').slice(0, 2).toUpperCase() || 'U',
  );

  save(): void {
    // TODO: PUT /me once authentication is wired up.
    this.saved.set(true);
    setTimeout(() => this.saved.set(false), 2000);
  }
}
