-- Requirement tags on states and transitions (ADR 0021): backlog URLs or short keys.
ALTER TABLE states ADD COLUMN requirements jsonb NOT NULL DEFAULT '[]';
ALTER TABLE transitions ADD COLUMN requirements jsonb NOT NULL DEFAULT '[]';
