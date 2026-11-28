DROP INDEX IF EXISTS tm_proposals_decided_idx;
DROP INDEX IF EXISTS proposals_decided_idx;
DROP INDEX IF EXISTS jobs_finished_idx;
DROP TRIGGER IF EXISTS tm_proposals_decided ON tm_proposals;
DROP TRIGGER IF EXISTS proposals_decided ON proposals;
DROP FUNCTION IF EXISTS set_decided_at();
ALTER TABLE tm_proposals DROP COLUMN IF EXISTS decided_at;
ALTER TABLE proposals DROP COLUMN IF EXISTS decided_at;
