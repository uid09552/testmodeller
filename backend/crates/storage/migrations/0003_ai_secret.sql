-- Persist the AI provider's API key so it survives a restart.
-- See docs/specification/07-ai-integration.md.
--
-- Stored encrypted, never in plain text: the column holds a ChaCha20-Poly1305
-- ciphertext and its nonce, sealed with a key the server reads from
-- TM_SECRET_KEY. A database dump therefore does not leak the provider key.

ALTER TABLE ai_settings ADD COLUMN api_key_nonce      bytea;
ALTER TABLE ai_settings ADD COLUMN api_key_ciphertext bytea;
