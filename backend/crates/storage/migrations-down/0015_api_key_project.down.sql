DROP INDEX IF EXISTS api_keys_project_idx;
ALTER TABLE api_keys DROP COLUMN IF EXISTS project_id;
