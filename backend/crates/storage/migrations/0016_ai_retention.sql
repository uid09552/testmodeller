-- Retention of AI history (ADR 0019): finished jobs and decided proposals are purged after
-- a configurable time. `decided_at` records when a proposal left `pending`.
ALTER TABLE proposals ADD COLUMN decided_at timestamptz;
ALTER TABLE tm_proposals ADD COLUMN decided_at timestamptz;
UPDATE proposals SET decided_at = created_at WHERE status <> 'pending';
UPDATE tm_proposals SET decided_at = created_at WHERE status <> 'pending';

CREATE FUNCTION set_decided_at() RETURNS trigger AS $$
BEGIN
    IF OLD.status = 'pending' AND NEW.status <> 'pending' THEN
        NEW.decided_at := now();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER proposals_decided BEFORE UPDATE OF status ON proposals
    FOR EACH ROW EXECUTE FUNCTION set_decided_at();
CREATE TRIGGER tm_proposals_decided BEFORE UPDATE OF status ON tm_proposals
    FOR EACH ROW EXECUTE FUNCTION set_decided_at();

CREATE INDEX jobs_finished_idx ON jobs (updated_at) WHERE status IN ('succeeded', 'failed');
CREATE INDEX proposals_decided_idx ON proposals (decided_at) WHERE status <> 'pending';
CREATE INDEX tm_proposals_decided_idx ON tm_proposals (decided_at) WHERE status <> 'pending';
