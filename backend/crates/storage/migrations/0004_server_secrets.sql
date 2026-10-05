-- Server-side data key for secrets stored in the database.
-- See docs/specification/07-ai-integration.md.
--
-- Used only when TM_SECRET_KEY is not configured: the server generates a key
-- once and keeps it here, so the AI provider key is persisted and survives a
-- restart out of the box. The key sitting beside the ciphertext means this
-- protects against a leaked row, a log line or an API response — not against
-- someone holding a full database dump. TM_SECRET_KEY is what protects
-- against that, and the server says which mode it is in at startup.

CREATE TABLE server_secrets (
    id          boolean PRIMARY KEY DEFAULT true CHECK (id),
    data_key    bytea NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
