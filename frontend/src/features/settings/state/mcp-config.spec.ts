import { daysLeft, mcpUrl, vscodeMcpConfig } from './mcp-config';

describe('mcp-config', () => {
  it('builds the endpoint URL from the origin', () => {
    expect(mcpUrl('https://tm.example.com')).toBe('https://tm.example.com/api/v1/mcp');
    expect(mcpUrl('http://localhost:4200/')).toBe('http://localhost:4200/api/v1/mcp');
  });

  it('writes a VS Code config that prompts for the key instead of containing it', () => {
    const config = JSON.parse(vscodeMcpConfig('https://tm.example.com'));
    const server = config.servers.testmodeller;
    expect(server.type).toBe('http');
    expect(server.url).toBe('https://tm.example.com/api/v1/mcp');
    expect(server.headers.Authorization).toBe('Bearer ${input:testmodeller-api-key}');
    expect(config.inputs[0]).toMatchObject({
      type: 'promptString', id: 'testmodeller-api-key', password: true,
    });
    expect(vscodeMcpConfig('https://x')).not.toContain('tm_');
  });

  it('counts whole days left', () => {
    const now = Date.parse('2026-01-01T00:00:00Z');
    expect(daysLeft('2026-01-31T00:00:00Z', now)).toBe(30);
    expect(daysLeft('2025-12-01T00:00:00Z', now)).toBe(0);
    expect(daysLeft(null, now)).toBeNull();
  });
});
