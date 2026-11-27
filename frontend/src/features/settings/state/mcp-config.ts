/** Path of the MCP endpoint, under the API base (05-api.md → MCP tools). */
export const MCP_PATH = '/api/v1/mcp';

/** Name of the VS Code input that prompts for the key. */
export const KEY_INPUT_ID = 'testmodeller-api-key';

/** The MCP endpoint URL for an app served from `origin`. */
export function mcpUrl(origin: string): string {
  return `${origin.replace(/\/+$/, '')}${MCP_PATH}`;
}

/**
 * The `.vscode/mcp.json` that connects VS Code to this server.
 *
 * The key is not written into the file: VS Code asks for it once through a
 * password input and keeps it in its secret storage, so the file is safe to
 * commit.
 */
export function vscodeMcpConfig(origin: string): string {
  const config = {
    inputs: [
      {
        type: 'promptString',
        id: KEY_INPUT_ID,
        description: 'TestModeller API key (Settings → MCP access)',
        password: true,
      },
    ],
    servers: {
      testmodeller: {
        type: 'http',
        url: mcpUrl(origin),
        headers: { Authorization: `Bearer \${input:${KEY_INPUT_ID}}` },
      },
    },
  };
  return JSON.stringify(config, null, 2);
}

/** Whole days from now until `iso`, or null without an expiry. */
export function daysLeft(iso: string | null | undefined, now = Date.now()): number | null {
  if (!iso) return null;
  return Math.max(0, Math.ceil((Date.parse(iso) - now) / 86_400_000));
}
