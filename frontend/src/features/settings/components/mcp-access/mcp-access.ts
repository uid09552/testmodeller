import {
  ChangeDetectionStrategy, Component, computed, inject, OnInit, signal,
} from '@angular/core';
import { DatePipe } from '@angular/common';
import {
  IxButton, IxCard, IxCardContent, IxIconButton, IxInput, IxMessageBar, IxPill, IxSelect,
  IxSelectItem, MessageService, ToastService,
} from '@siemens/ix-angular/standalone';
import { iconCopy, iconPlus, iconTrashcan } from '@siemens/ix-icons/icons';
import { ApiKeysApi } from '../../../../core/api/api-keys-api';
import { ApiError } from '../../../../core/api/api-error';
import { ApiKey, ApiKeyCreated } from '../../../../core/api/api.types';
import { ExplorerStore } from '../../../explorer/state/explorer.store';
import { daysLeft, mcpUrl, uploadActionSnippet, vscodeMcpConfig } from '../../state/mcp-config';

/** `Use for` value of a key for AI clients, as opposed to a project id. */
export const FOR_MCP = 'mcp';

/** Expiry choices offered when creating a key; the API allows 1–365 days. */
export const EXPIRY_DAYS = [30, 90, 180, 365] as const;
const DEFAULT_DAYS = 90;

/**
 * Settings → MCP access (FR-090, FR-092): the caller's API keys and a
 * copyable VS Code configuration. A new key is shown exactly once.
 */
@Component({
  selector: 'tm-mcp-access',
  imports: [
    DatePipe, IxButton, IxCard, IxCardContent, IxIconButton, IxInput, IxMessageBar, IxPill,
    IxSelect, IxSelectItem,
  ],
  templateUrl: './mcp-access.html',
  styleUrl: './mcp-access.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class McpAccessComponent implements OnInit {
  private readonly api = inject(ApiKeysApi);
  private readonly toasts = inject(ToastService);
  private readonly messages = inject(MessageService);
  private readonly explorer = inject(ExplorerStore);

  readonly projects = this.explorer.projects;
  /** What a new key is for: `mcp`, or the id of the project whose results it uploads. */
  readonly forWhat = signal<string>(FOR_MCP);
  readonly icons = { copy: iconCopy, add: iconPlus, revoke: iconTrashcan };
  readonly expiryDays = EXPIRY_DAYS;

  readonly keys = signal<ApiKey[]>([]);
  readonly loading = signal(true);
  readonly busy = signal(false);
  readonly error = signal<string | null>(null);
  /** The key just created; cleared when the user is done with it. */
  readonly created = signal<ApiKeyCreated | null>(null);

  readonly name = signal('');
  readonly days = signal<number>(DEFAULT_DAYS);
  /** Project of an upload key, by id, for the list. */
  projectName(id: string | null | undefined): string {
    return this.projects().find(p => p.id === id)?.name ?? 'a project';
  }

  setFor(event: Event): void {
    const v = (event as CustomEvent<string | string[]>).detail;
    const value = Array.isArray(v) ? v[0] : v;
    this.forWhat.set(value === FOR_MCP || this.projects().some(p => p.id === value) ? value : FOR_MCP);
  }

  /** The GitHub Action step for the key just created, when it is an upload key. */
  readonly actionSnippet = computed(() => {
    const c = this.created();
    return c?.projectId ? uploadActionSnippet(this.origin, c.projectId) : null;
  });

  readonly canCreate = computed(() => this.name().trim().length > 0 && !this.busy());

  private readonly origin = typeof location === 'undefined' ? '' : location.origin;
  readonly url = computed(() => mcpUrl(this.origin));
  readonly config = computed(() => vscodeMcpConfig(this.origin));

  async ngOnInit(): Promise<void> {
    await this.load();
  }

  async load(): Promise<void> {
    this.loading.set(true);
    try {
      this.keys.set(await this.api.list());
      this.error.set(null);
    } catch (e) {
      this.error.set(message(e, 'Cannot load your API keys.'));
    } finally {
      this.loading.set(false);
    }
  }

  setName(event: Event): void {
    this.name.set(String((event as CustomEvent<string>).detail ?? ''));
  }

  setDays(event: Event): void {
    const value = Number((event as CustomEvent<string>).detail);
    if (EXPIRY_DAYS.includes(value as (typeof EXPIRY_DAYS)[number])) this.days.set(value);
  }

  async create(): Promise<void> {
    if (!this.canCreate()) return;
    this.busy.set(true);
    try {
      const created = await this.api.create({
        name: this.name().trim(),
        expiresInDays: this.days(),
        ...(this.forWhat() !== FOR_MCP ? { projectId: this.forWhat() } : {}),
      });
      this.created.set(created);
      this.name.set('');
      const { key: _secret, ...listed } = created;
      this.keys.update(list => [listed, ...list]);
      this.error.set(null);
    } catch (e) {
      this.error.set(message(e, 'The key could not be created.'));
    } finally {
      this.busy.set(false);
    }
  }

  async revoke(key: ApiKey): Promise<void> {
    if (!(await this.confirmRevoke(key))) return;
    try {
      await this.api.revoke(key.id);
      this.keys.update(list => list.filter(k => k.id !== key.id));
      if (this.created()?.id === key.id) this.created.set(null);
      void this.toasts.show({ type: 'success', message: `Revoked "${key.name}".` });
    } catch (e) {
      this.error.set(message(e, 'The key could not be revoked.'));
    }
  }

  /** Destructive, so it asks first (DESIGN.md → Buttons). */
  confirmRevoke(key: ApiKey): Promise<boolean> {
    return new Promise(resolve => {
      void this.messages
        .danger(
          'Revoke API key?',
          `"${key.name}" (${key.prefix}…) stops working at once. Clients using it lose access.`,
          'Revoke', 'Cancel', 'revoke', 'cancel',
        )
        .then(event => event.once(result => resolve(result.payload === 'revoke')));
    });
  }

  async copy(text: string, what: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      void this.toasts.show({ type: 'success', message: `${what} copied.` });
    } catch {
      void this.toasts.show({ type: 'warning', message: `Copy failed; select the ${what.toLowerCase()} and copy it by hand.` });
    }
  }

  daysLeft(iso: string | null | undefined): number | null {
    return daysLeft(iso);
  }
}

function message(e: unknown, fallback: string): string {
  return e instanceof ApiError ? e.message : fallback;
}
