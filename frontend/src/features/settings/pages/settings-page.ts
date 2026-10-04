import { ChangeDetectionStrategy, Component, signal } from '@angular/core';

@Component({
  selector: 'tm-settings-page',
  templateUrl: './settings-page.html',
  styleUrl: './settings-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SettingsPageComponent {
  // ── AI provider ────────────────────────────────────────────────────────────
  readonly provider = signal<'anthropic' | 'openai' | 'none'>('anthropic');
  readonly model    = signal('claude-opus-5-5');

  /**
   * Held in memory for the lifetime of the page only — never written to
   * localStorage and never sent anywhere except the backend's configure call,
   * which keeps it in memory too (AGENTS.md: keys are never persisted).
   */
  readonly apiKey   = signal('');
  readonly keyVisible = signal(false);
  readonly keySaved   = signal(false);

  // ── Generation defaults ────────────────────────────────────────────────────
  readonly criterion = signal<'all-states' | 'all-transitions' | 'all-paths'>('all-transitions');
  readonly maxPathLength = signal(12);
  readonly seed = signal('');

  // ── Toggles ───────────────────────────────────────────────────────────────
  readonly autoValidate   = signal(true);
  readonly snapToGrid     = signal(false);
  readonly confirmDeletes = signal(true);
  readonly aiEnabled      = signal(true);

  toggleKeyVisible(): void { this.keyVisible.update(v => !v); }

  saveKey(): void {
    // TODO: POST the key to the backend's in-memory config endpoint.
    this.keySaved.set(true);
    setTimeout(() => this.keySaved.set(false), 2000);
  }

  clearKey(): void {
    this.apiKey.set('');
    this.keySaved.set(false);
  }

  toggle(s: ReturnType<typeof signal<boolean>>): void { s.update(v => !v); }
}
