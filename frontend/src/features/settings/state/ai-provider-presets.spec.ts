import { choiceFor, envValueFor, providerLabel } from './ai-provider-presets';

describe('choiceFor', () => {
  it('recognises what was stored', () => {
    expect(choiceFor('anthropic', '')).toBe('claude');
    expect(choiceFor('local', 'http://localhost:11434/v1')).toBe('ollama');
    expect(choiceFor('openai-compatible', 'https://api.example.com/v1'))
      .toBe('openai-compatible');
    expect(choiceFor('none', '')).toBe('none');
    expect(choiceFor(undefined, '')).toBe('none');
  });

  it('reads an openai-compatible setting on Ollama s port back as Ollama', () => {
    // Both speak the same protocol, so the endpoint is the only clue.
    expect(choiceFor('openai-compatible', 'http://ollama:11434/v1')).toBe('ollama');
  });
});

describe('labels', () => {
  it('names every provider', () => {
    expect(providerLabel('claude')).toBe('Claude (Anthropic)');
    expect(providerLabel('ollama')).toBe('Ollama');
    expect(providerLabel('none')).toContain('disabled');
  });

  it('gives the TM_AI_PROVIDER value the backend accepts', () => {
    // Must match the aliases parsed in backend/crates/api/src/config.rs.
    expect(envValueFor('claude')).toBe('claude');
    expect(envValueFor('ollama')).toBe('ollama');
    expect(envValueFor('openai-compatible')).toBe('openai-compatible');
  });
});

describe('SettingsPageComponent env example', () => {
  async function pageWith(settings: object) {
    const { TestBed } = await import('@angular/core/testing');
    const { provideHttpClient } = await import('@angular/common/http');
    const { SettingsPageComponent } = await import('../pages/settings-page');
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [provideHttpClient()] });
    const page = TestBed.createComponent(SettingsPageComponent).componentInstance;
    page.settings.set(settings);
    return page;
  }

  it('never suggests an Ollama endpoint for a Claude setup', async () => {
    const page = await pageWith({ provider: 'anthropic', model: 'claude-opus-5-5' });
    const example = page.envExample();

    expect(example).toContain('TM_AI_PROVIDER=claude');
    expect(example).not.toContain('11434');
    expect(example).toContain('required');
  });

  it('reproduces an Ollama setup, without asking for a key', async () => {
    const page = await pageWith({
      provider: 'local', model: 'llama3.1', baseUrl: 'http://ollama:11434/v1',
    });
    const example = page.envExample();

    expect(example).toContain('TM_AI_PROVIDER=ollama');
    expect(example).toContain('TM_AI_BASE_URL=http://ollama:11434/v1');
    expect(example).toContain('not needed');
  });

  it('offers a working Ollama starting point when AI is off', async () => {
    const page = await pageWith({ provider: 'none' });
    expect(page.envExample()).toContain('TM_AI_PROVIDER=ollama');
  });
});
