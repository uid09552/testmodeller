import {
  ChangeDetectionStrategy, Component, computed, inject, OnInit, signal,
} from '@angular/core';
import { AiApi } from '../../../core/api/ai-api';
import { ApiError } from '../../../core/api/api-error';
import { AiSettings } from '../../../core/api/api.types';
import {
  choiceFor, envValueFor, providerLabel,
} from '../state/ai-provider-presets';

@Component({
  selector: 'tm-settings-page',
  templateUrl: './settings-page.html',
  styleUrl: './settings-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SettingsPageComponent implements OnInit {
  private readonly api = inject(AiApi);

  // ── AI provider ────────────────────────────────────────────────────────────
  // Configured in .env and passed to the backend by compose.yaml, not here
  // (07-ai-integration.md). The page shows what is in effect and how to change
  // it, so nobody edits a form that the next restart would overwrite.
  readonly settings = signal<AiSettings | null>(null);
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);

  readonly choice = computed(() => {
    const s = this.settings();
    return choiceFor(s?.provider, s?.baseUrl ?? '');
  });
  readonly providerLabel = computed(() => providerLabel(this.choice()));
  readonly envProvider = computed(() => envValueFor(this.choice()));

  /**
   * The `.env` lines that reproduce what is in effect, or a working Ollama
   * starting point when AI is off. Each line follows the provider, so a Claude
   * setup is never shown an Ollama endpoint.
   */
  readonly envExample = computed(() => {
    const s = this.settings();
    const choice = this.choice();
    if (choice === 'none') {
      return [
        'TM_AI_PROVIDER=ollama',
        'TM_AI_MODEL=llama3.1',
        'TM_AI_BASE_URL=http://host.docker.internal:11434/v1',
        'TM_AI_API_KEY=          # not needed for a local Ollama',
      ].join('\n');
    }
    const baseUrl = s?.baseUrl
      ? `TM_AI_BASE_URL=${s.baseUrl}`
      : 'TM_AI_BASE_URL=         # empty: the provider default';
    const key = choice === 'ollama'
      ? 'TM_AI_API_KEY=          # not needed for a local Ollama'
      : 'TM_AI_API_KEY=…         # required; never shown here';
    return [
      `TM_AI_PROVIDER=${this.envProvider()}`,
      `TM_AI_MODEL=${s?.model ?? ''}`,
      baseUrl,
      key,
    ].join('\n');
  });

  /** Whether the provider can actually be called, as far as Settings can tell. */
  readonly configured = computed(() => {
    const s = this.settings();
    if (!s || this.choice() === 'none') return false;
    // A key is only optional for a local Ollama.
    return this.choice() === 'ollama' || !!s.secretConfigured;
  });

  async ngOnInit(): Promise<void> {
    try {
      this.settings.set(await this.api.getSettings());
    } catch (e) {
      this.error.set(e instanceof ApiError ? e.message : 'Cannot load the AI settings.');
    } finally {
      this.loading.set(false);
    }
  }

  // ── Generation defaults ────────────────────────────────────────────────────
  readonly criterion = signal<'all-states' | 'all-transitions' | 'all-paths'>('all-transitions');
  readonly maxPathLength = signal(12);
  readonly seed = signal('');

  // ── Toggles ───────────────────────────────────────────────────────────────
  readonly autoValidate   = signal(true);
  readonly snapToGrid     = signal(false);
  readonly confirmDeletes = signal(true);

  toggle(s: ReturnType<typeof signal<boolean>>): void { s.update(v => !v); }
}
