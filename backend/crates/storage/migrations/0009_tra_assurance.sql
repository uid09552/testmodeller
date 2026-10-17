-- Threat model assurance: residual risk, acceptance expiry, evidence, baselines and
-- detail fields. Additive only; see docs/specification/09-threat-modeling.md.

ALTER TABLE tm_threats
    ADD COLUMN residual_likelihood smallint CHECK (residual_likelihood BETWEEN 1 AND 5),
    ADD COLUMN residual_impact     smallint CHECK (residual_impact BETWEEN 1 AND 5),
    ADD COLUMN threat_source       text,
    ADD COLUMN detection           text,
    ADD COLUMN existing_mitigations text,
    ADD COLUMN confidence          text CHECK (confidence IN ('low', 'medium', 'high')),
    ADD COLUMN acceptance_expires  date,
    ADD COLUMN accepted_on         date,
    ADD CONSTRAINT tm_threats_residual_pair CHECK ((residual_likelihood IS NULL) = (residual_impact IS NULL));

ALTER TABLE tm_assets
    ADD COLUMN classification     text CHECK (classification IN ('public', 'internal', 'confidential', 'restricted')),
    ADD COLUMN data_location      text,
    ADD COLUMN retention          text,
    ADD COLUMN encryption_at_rest text NOT NULL DEFAULT 'unknown' CHECK (encryption_at_rest IN ('yes', 'no', 'unknown'));

ALTER TABLE tm_flows
    ADD COLUMN data_classification text CHECK (data_classification IN ('public', 'internal', 'confidential', 'restricted')),
    ADD COLUMN authorization_model text,
    ADD COLUMN entry_point         boolean NOT NULL DEFAULT false;

ALTER TABLE tm_controls
    ADD COLUMN review_by date;

-- Evidence is a reference (link and/or note), owned by a control or a test result.
CREATE TABLE tm_evidence (
    id              uuid PRIMARY KEY,
    threat_model_id uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    control_id      uuid REFERENCES tm_controls (id) ON DELETE CASCADE,
    result_id       uuid REFERENCES security_test_results (id) ON DELETE CASCADE,
    kind            text NOT NULL CHECK (kind IN ('document', 'test-report', 'scan', 'configuration',
                        'review', 'other')),
    title           text NOT NULL,
    link            text,
    note            text,
    evidence_date   date NOT NULL,
    reviewer        text,
    version         integer NOT NULL DEFAULT 1,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CHECK ((control_id IS NULL) <> (result_id IS NULL)),
    CHECK (link IS NOT NULL OR note IS NOT NULL)
);
CREATE INDEX tm_evidence_tm_idx ON tm_evidence (threat_model_id);
CREATE INDEX tm_evidence_control_idx ON tm_evidence (control_id) WHERE control_id IS NOT NULL;
CREATE INDEX tm_evidence_result_idx ON tm_evidence (result_id) WHERE result_id IS NOT NULL;

-- Immutable snapshot of a threat model; the snapshot is the read model as JSON.
CREATE TABLE tm_baselines (
    id              uuid PRIMARY KEY,
    threat_model_id uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    name            text NOT NULL,
    note            text,
    created_by      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    schema_version  integer NOT NULL,
    threat_count    integer NOT NULL,
    control_count   integer NOT NULL,
    snapshot        jsonb NOT NULL,
    UNIQUE (threat_model_id, name)
);
CREATE INDEX tm_baselines_tm_idx ON tm_baselines (threat_model_id, created_at DESC);
