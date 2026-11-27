-- Per-user API keys for the MCP endpoint. Only a SHA-256 hash of the key is stored.
-- See docs/adr/0015-api-keys-and-mcp.md and docs/specification/08-usermanagement.md.

CREATE TABLE api_keys (
    id           uuid PRIMARY KEY,
    tenant_id    text NOT NULL,
    subject      text NOT NULL,
    name         text NOT NULL CHECK (length(name) BETWEEN 1 AND 80),
    prefix       text NOT NULL,
    key_hash     bytea NOT NULL UNIQUE CHECK (length(key_hash) = 32),
    role         text NOT NULL CHECK (role IN ('User', 'Editor')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz,
    last_used_at timestamptz,
    revoked_at   timestamptz
);

CREATE INDEX api_keys_owner ON api_keys (tenant_id, subject) WHERE revoked_at IS NULL;
