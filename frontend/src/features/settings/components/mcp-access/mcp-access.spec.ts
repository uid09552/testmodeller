import { CUSTOM_ELEMENTS_SCHEMA } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import {
  IxButton, IxCard, IxCardContent, IxIconButton, IxInput, IxMessageBar, IxPill, IxSelect,
  IxSelectItem, MessageService, ToastService,
} from '@siemens/ix-angular/standalone';
import { ApiKeysApi } from '../../../../core/api/api-keys-api';
import { ApiKey, ApiKeyCreated } from '../../../../core/api/api.types';
import { McpAccessComponent } from './mcp-access';

/** The iX wrappers start real web components; here they are inert elements. */
const INERT = {
  remove: { imports: [IxButton, IxCard, IxCardContent, IxIconButton, IxInput, IxMessageBar, IxPill, IxSelect, IxSelectItem] },
  add: { schemas: [CUSTOM_ELEMENTS_SCHEMA] },
};

const KEY: ApiKey = {
  id: 'k1', name: 'Laptop', prefix: 'tm_abc1234', role: 'Editor',
  createdAt: '2026-11-01T00:00:00Z', expiresAt: '2027-01-30T00:00:00Z', lastUsedAt: null,
};

// Importing the iX package registers its custom elements, which then upgrade
// in jsdom; some observe visibility, an API jsdom does not have.
class NoIntersectionObserver {
  observe(): void { /* jsdom has no layout */ }
  unobserve(): void { /* nothing observed */ }
  disconnect(): void { /* nothing observed */ }
  takeRecords(): [] { return []; }
}

describe('McpAccessComponent', () => {
  beforeAll(() => {
    (globalThis as { IntersectionObserver?: unknown }).IntersectionObserver ??= NoIntersectionObserver;
  });

  let api: { list: ReturnType<typeof vi.fn>; create: ReturnType<typeof vi.fn>; revoke: ReturnType<typeof vi.fn> };
  let toasts: { show: ReturnType<typeof vi.fn> };

  async function render(keys: ApiKey[] = [KEY]) {
    api = {
      list: vi.fn().mockResolvedValue(keys),
      create: vi.fn(),
      revoke: vi.fn().mockResolvedValue(undefined),
    };
    toasts = { show: vi.fn().mockResolvedValue(undefined) };
    TestBed.configureTestingModule({
      imports: [McpAccessComponent],
      providers: [
        { provide: ApiKeysApi, useValue: api },
        { provide: ToastService, useValue: toasts },
        { provide: MessageService, useValue: {} },
      ],
    });
    TestBed.overrideComponent(McpAccessComponent, INERT);
    const f = TestBed.createComponent(McpAccessComponent);
    f.detectChanges();
    await f.whenStable();
    f.detectChanges();
    return f;
  }

  it('lists keys by name and prefix, never a secret', async () => {
    const f = await render();
    const text = (f.nativeElement as HTMLElement).textContent ?? '';
    expect(text).toContain('Laptop');
    expect(text).toContain('tm_abc1234…');
    expect(text).toContain('Read & write');
    expect(f.nativeElement.querySelector('[data-testid="new-key"]')).toBeNull();
  });

  it('shows the VS Code configuration for this origin with a prompted key', async () => {
    const f = await render([]);
    const code = (f.nativeElement as HTMLElement).querySelector('pre')?.textContent ?? '';
    const config = JSON.parse(code);
    expect(config.servers.testmodeller.url).toBe(`${location.origin}/api/v1/mcp`);
    expect(config.servers.testmodeller.headers.Authorization).toBe('Bearer ${input:testmodeller-api-key}');
    expect((f.nativeElement as HTMLElement).textContent).toContain('No keys yet.');
  });

  it('creates a key, shows it once, and lists it without the secret', async () => {
    const f = await render([]);
    const created: ApiKeyCreated = { ...KEY, id: 'k2', name: 'CI', key: 'tm_' + 'a'.repeat(64) };
    api.create.mockResolvedValue(created);
    const c = f.componentInstance;

    expect(c.canCreate()).toBe(false);
    c.setName(new CustomEvent('valueChange', { detail: '  CI  ' }));
    c.setDays(new CustomEvent('valueChange', { detail: '30' }));
    await c.create();
    f.detectChanges();

    expect(api.create).toHaveBeenCalledWith({ name: 'CI', expiresInDays: 30 });
    const shown = f.nativeElement.querySelector('[data-testid="new-key"]');
    expect(shown?.textContent).toBe(created.key);
    expect(c.keys()[0]).not.toHaveProperty('key');
    expect(c.name()).toBe('');

    c.created.set(null);
    f.detectChanges();
    expect(f.nativeElement.querySelector('[data-testid="new-key"]')).toBeNull();
  });

  it('ignores an expiry the API would reject', async () => {
    const f = await render([]);
    f.componentInstance.setDays(new CustomEvent('valueChange', { detail: '9999' }));
    expect(f.componentInstance.days()).toBe(90);
  });

  it('revokes only after confirmation', async () => {
    const f = await render();
    const c = f.componentInstance;

    vi.spyOn(c, 'confirmRevoke').mockResolvedValue(false);
    await c.revoke(KEY);
    expect(api.revoke).not.toHaveBeenCalled();
    expect(c.keys()).toHaveLength(1);

    vi.spyOn(c, 'confirmRevoke').mockResolvedValue(true);
    await c.revoke(KEY);
    expect(api.revoke).toHaveBeenCalledWith('k1');
    expect(c.keys()).toHaveLength(0);
    expect(toasts.show).toHaveBeenCalled();
  });

  it('reports a failed load', async () => {
    api = { list: vi.fn(), create: vi.fn(), revoke: vi.fn() };
    TestBed.configureTestingModule({
      imports: [McpAccessComponent],
      providers: [
        { provide: ApiKeysApi, useValue: { ...api, list: vi.fn().mockRejectedValue(new Error('x')) } },
        { provide: ToastService, useValue: { show: vi.fn() } },
        { provide: MessageService, useValue: {} },
      ],
    });
    TestBed.overrideComponent(McpAccessComponent, INERT);
    const f = TestBed.createComponent(McpAccessComponent);
    f.detectChanges();
    await f.whenStable();
    expect(f.componentInstance.error()).toBe('Cannot load your API keys.');
  });
});
