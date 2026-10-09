-- The editor's layout of a model: an opaque JSON object owned by the frontend
-- (ADR 0010). Absent for models saved before it existed; they show defaults.
ALTER TABLE models ADD COLUMN layout jsonb;
