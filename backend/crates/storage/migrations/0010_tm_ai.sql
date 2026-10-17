-- AI assistance for threat models: opt-in flag, provenance of AI-created entities and the
-- store of pending proposals. Additive only; see docs/adr/0013-threat-model-proposals-store.md.

ALTER TABLE threat_models ADD COLUMN ai_enabled boolean NOT NULL DEFAULT false;

ALTER TABLE tm_zones      ADD COLUMN origin text NOT NULL DEFAULT 'manual' CHECK (origin IN ('manual', 'ai')), ADD COLUMN ai_source text;
ALTER TABLE tm_components ADD COLUMN origin text NOT NULL DEFAULT 'manual' CHECK (origin IN ('manual', 'ai')), ADD COLUMN ai_source text;
ALTER TABLE tm_assets     ADD COLUMN origin text NOT NULL DEFAULT 'manual' CHECK (origin IN ('manual', 'ai')), ADD COLUMN ai_source text;
ALTER TABLE tm_flows      ADD COLUMN origin text NOT NULL DEFAULT 'manual' CHECK (origin IN ('manual', 'ai')), ADD COLUMN ai_source text;
ALTER TABLE tm_controls   ADD COLUMN origin text NOT NULL DEFAULT 'manual' CHECK (origin IN ('manual', 'ai')), ADD COLUMN ai_source text;

ALTER TABLE tm_threats DROP CONSTRAINT tm_threats_origin_check;
ALTER TABLE tm_threats ADD CONSTRAINT tm_threats_origin_check CHECK (origin IN ('manual', 'rule', 'ai'));
ALTER TABLE tm_threats ADD COLUMN ai_source text;

ALTER TABLE security_tests DROP CONSTRAINT security_tests_origin_check;
ALTER TABLE security_tests ADD CONSTRAINT security_tests_origin_check CHECK (origin IN ('manual', 'generated', 'ai'));
ALTER TABLE security_tests ADD COLUMN ai_source text;

CREATE TABLE tm_proposals (
    id                  uuid PRIMARY KEY,
    threat_model_id     uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    kind                text NOT NULL CHECK (kind IN ('tm-architecture', 'tm-threats', 'tm-ratings',
                            'tm-controls', 'tm-security-tests', 'tm-review')),
    status              text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'rejected')),
    payload             jsonb NOT NULL,
    rationale           text,
    source              text,
    prompt_version      text,
    target_ids          uuid[] NOT NULL DEFAULT '{}',
    resulting_entity_ids uuid[] NOT NULL DEFAULT '{}',
    reject_reason       text,
    created_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX tm_proposals_tm_idx ON tm_proposals (threat_model_id, created_at DESC, id);
