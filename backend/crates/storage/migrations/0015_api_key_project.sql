-- An upload key (ADR 0018) belongs to one project and may only post its test results.
-- Deleting the project deletes its upload keys.
ALTER TABLE api_keys ADD COLUMN project_id uuid REFERENCES projects (id) ON DELETE CASCADE;
CREATE INDEX api_keys_project_idx ON api_keys (project_id) WHERE project_id IS NOT NULL;
