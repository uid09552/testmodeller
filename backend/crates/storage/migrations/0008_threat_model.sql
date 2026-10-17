-- Threat modeling (FR-060..FR-076). See docs/specification/09-threat-modeling.md
-- and docs/adr/0011-threat-model-entities-and-security-tests.md.
--
-- One threat model per project. Everything below hangs off it and is deleted
-- with it. Computed values (crossings, warnings, levels, flags, staleness) are
-- not stored. The tenant is copied from the project, as for test results.

CREATE TABLE threat_models (
    id              uuid PRIMARY KEY,
    project_id      uuid NOT NULL UNIQUE REFERENCES projects (id) ON DELETE CASCADE,
    tenant_id       text NOT NULL,
    name            text NOT NULL,
    scope           text,
    objectives      text,
    assumptions     text,
    exclusions      text,
    thresholds      jsonb NOT NULL,
    layout          jsonb,
    layout_version  integer NOT NULL DEFAULT 0,
    next_threat_no  integer NOT NULL DEFAULT 1,
    next_control_no integer NOT NULL DEFAULT 1,
    next_test_no    integer NOT NULL DEFAULT 1,
    version         integer NOT NULL DEFAULT 1,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tm_zones (
    id              uuid PRIMARY KEY,
    threat_model_id uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    name            text NOT NULL,
    description     text,
    kind            text NOT NULL CHECK (kind IN ('internet', 'partner', 'dmz', 'internal',
                        'restricted-data', 'management', 'identity', 'cicd', 'third-party', 'other')),
    trust_level     smallint NOT NULL CHECK (trust_level BETWEEN 0 AND 4),
    version         integer NOT NULL DEFAULT 1,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE (threat_model_id, name)
);

CREATE TABLE tm_assets (
    id              uuid PRIMARY KEY,
    threat_model_id uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    name            text NOT NULL,
    description     text,
    category        text NOT NULL CHECK (category IN ('business-data', 'identity-secrets',
                        'software-code', 'infrastructure', 'security-asset', 'operational-asset',
                        'business-capability')),
    owner           text,
    confidentiality text CHECK (confidentiality IN ('low', 'medium', 'high')),
    integrity       text CHECK (integrity IN ('low', 'medium', 'high')),
    availability    text CHECK (availability IN ('low', 'medium', 'high')),
    personal_data   boolean NOT NULL DEFAULT false,
    version         integer NOT NULL DEFAULT 1,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE (threat_model_id, name)
);

CREATE TABLE tm_components (
    id                  uuid PRIMARY KEY,
    threat_model_id     uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    name                text NOT NULL,
    description         text,
    component_type      text NOT NULL CHECK (component_type IN ('web-frontend', 'mobile-app',
                            'desktop-client', 'browser-extension', 'api-gateway', 'api',
                            'message-broker', 'webhook', 'external-connector', 'service', 'worker',
                            'scheduler', 'serverless-function', 'relational-database',
                            'object-storage', 'cache', 'search-index', 'file-store', 'backup',
                            'identity-provider', 'authorization-service', 'secrets-manager',
                            'key-management', 'audit-service', 'cloud-service',
                            'container-platform', 'cicd-pipeline', 'third-party-saas', 'library',
                            'human-user', 'external-system')),
    owner               text,
    technology          text,
    criticality         text CHECK (criticality IN ('low', 'medium', 'high', 'critical')),
    internet_facing     boolean NOT NULL DEFAULT false,
    zone_id             uuid REFERENCES tm_zones (id) ON DELETE SET NULL,
    linked_component_id uuid REFERENCES components (id) ON DELETE SET NULL,
    linked_feature_id   uuid REFERENCES features (id) ON DELETE SET NULL,
    version             integer NOT NULL DEFAULT 1,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    UNIQUE (threat_model_id, name)
);

CREATE TABLE tm_flows (
    id              uuid PRIMARY KEY,
    threat_model_id uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    source_id       uuid NOT NULL REFERENCES tm_components (id) ON DELETE CASCADE,
    target_id       uuid NOT NULL REFERENCES tm_components (id) ON DELETE CASCADE,
    relationship    text NOT NULL CHECK (relationship IN ('calls', 'sends-data', 'reads-writes',
                        'authenticates-through', 'trusts', 'depends-on', 'runs-on',
                        'publishes-subscribes', 'replicates-backs-up', 'deploys')),
    label           text,
    protocol        text,
    bidirectional   boolean NOT NULL DEFAULT false,
    authentication  text NOT NULL DEFAULT 'unknown',
    encrypted       text NOT NULL DEFAULT 'unknown' CHECK (encrypted IN ('yes', 'no', 'unknown')),
    version         integer NOT NULL DEFAULT 1,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tm_threats (
    id                   uuid PRIMARY KEY,
    threat_model_id      uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    number               integer NOT NULL,
    title                text NOT NULL,
    category             text NOT NULL CHECK (category IN ('spoofing', 'tampering', 'repudiation',
                             'information-disclosure', 'denial-of-service',
                             'elevation-of-privilege')),
    scenario             text,
    attacker_objective   text,
    preconditions        text,
    consequences         text,
    status               text NOT NULL CHECK (status IN ('candidate', 'open', 'mitigated',
                             'accepted', 'dismissed')),
    dismiss_reason       text,
    origin               text NOT NULL CHECK (origin IN ('manual', 'rule')),
    explanation          jsonb,
    inherent_likelihood  smallint CHECK (inherent_likelihood BETWEEN 1 AND 5),
    inherent_impact      smallint CHECK (inherent_impact BETWEEN 1 AND 5),
    current_likelihood   smallint CHECK (current_likelihood BETWEEN 1 AND 5),
    current_impact       smallint CHECK (current_impact BETWEEN 1 AND 5),
    treatment            text NOT NULL DEFAULT 'undecided' CHECK (treatment IN ('undecided',
                             'mitigate', 'avoid', 'transfer', 'accept')),
    risk_owner           text,
    due_date             date,
    acceptance_rationale text,
    accepted_by          text,
    version              integer NOT NULL DEFAULT 1,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),
    UNIQUE (threat_model_id, number),
    CHECK ((inherent_likelihood IS NULL) = (inherent_impact IS NULL)),
    CHECK ((current_likelihood IS NULL) = (current_impact IS NULL))
);

CREATE TABLE tm_controls (
    id                  uuid PRIMARY KEY,
    threat_model_id     uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    number              integer NOT NULL,
    name                text NOT NULL,
    description         text,
    control_type        text NOT NULL CHECK (control_type IN ('preventive', 'detective',
                            'corrective', 'compensating')),
    category            text NOT NULL CHECK (category IN ('identity-access', 'data-protection',
                            'application-network', 'detection-response', 'secure-development')),
    status              text NOT NULL CHECK (status IN ('planned', 'implemented', 'verified')),
    owner               text,
    acceptance_criteria text,
    verified_at         timestamptz,
    version             integer NOT NULL DEFAULT 1,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    UNIQUE (threat_model_id, number)
);

-- Rule keys outlive their threats, so a dismissed or deleted suggestion is
-- never suggested again.
CREATE TABLE tm_rule_keys (
    threat_model_id uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    rule_key        text NOT NULL,
    threat_id       uuid REFERENCES tm_threats (id) ON DELETE SET NULL,
    PRIMARY KEY (threat_model_id, rule_key)
);

CREATE TABLE security_tests (
    id                uuid PRIMARY KEY,
    threat_model_id   uuid NOT NULL REFERENCES threat_models (id) ON DELETE CASCADE,
    number            integer NOT NULL,
    title             text NOT NULL,
    objective         text,
    preconditions     text[] NOT NULL DEFAULT '{}',
    steps             jsonb NOT NULL DEFAULT '[]',
    test_type         text NOT NULL CHECK (test_type IN ('negative-authorization',
                          'authentication', 'input-validation', 'transport-security',
                          'configuration-review', 'resilience', 'logging-audit', 'other')),
    status            text NOT NULL CHECK (status IN ('draft', 'ready', 'obsolete')),
    origin            text NOT NULL CHECK (origin IN ('manual', 'generated')),
    -- Template source of a generated test. The threat and control ids are kept
    -- without a foreign key: they must survive deletion to explain staleness.
    template_id       text,
    template_version  integer,
    source_threat_id  uuid,
    source_control_id uuid,
    template_key      text,
    version           integer NOT NULL DEFAULT 1,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    UNIQUE (threat_model_id, number)
);
CREATE UNIQUE INDEX security_tests_template_key_idx ON security_tests (threat_model_id, template_key)
    WHERE template_key IS NOT NULL AND status <> 'obsolete';

CREATE TABLE security_test_results (
    id               uuid PRIMARY KEY,
    security_test_id uuid NOT NULL REFERENCES security_tests (id) ON DELETE CASCADE,
    outcome          text NOT NULL CHECK (outcome IN ('passed', 'failed', 'not-run')),
    note             text,
    executed_at      timestamptz NOT NULL,
    recorded_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX security_test_results_latest_idx
    ON security_test_results (security_test_id, executed_at DESC, recorded_at DESC);

-- Links. `position` keeps the order the client sent.
CREATE TABLE tm_component_assets (
    component_id uuid NOT NULL REFERENCES tm_components (id) ON DELETE CASCADE,
    asset_id     uuid NOT NULL REFERENCES tm_assets (id) ON DELETE CASCADE,
    position     integer NOT NULL,
    PRIMARY KEY (component_id, asset_id)
);
CREATE TABLE tm_flow_assets (
    flow_id  uuid NOT NULL REFERENCES tm_flows (id) ON DELETE CASCADE,
    asset_id uuid NOT NULL REFERENCES tm_assets (id) ON DELETE CASCADE,
    position integer NOT NULL,
    PRIMARY KEY (flow_id, asset_id)
);
CREATE TABLE tm_threat_targets (
    threat_id    uuid NOT NULL REFERENCES tm_threats (id) ON DELETE CASCADE,
    component_id uuid REFERENCES tm_components (id) ON DELETE CASCADE,
    flow_id      uuid REFERENCES tm_flows (id) ON DELETE CASCADE,
    position     integer NOT NULL,
    PRIMARY KEY (threat_id, position),
    CHECK (num_nonnulls(component_id, flow_id) = 1)
);
CREATE TABLE tm_threat_assets (
    threat_id uuid NOT NULL REFERENCES tm_threats (id) ON DELETE CASCADE,
    asset_id  uuid NOT NULL REFERENCES tm_assets (id) ON DELETE CASCADE,
    position  integer NOT NULL,
    PRIMARY KEY (threat_id, asset_id)
);
CREATE TABLE tm_threat_controls (
    threat_id  uuid NOT NULL REFERENCES tm_threats (id) ON DELETE CASCADE,
    control_id uuid NOT NULL REFERENCES tm_controls (id) ON DELETE CASCADE,
    position   integer NOT NULL,
    PRIMARY KEY (threat_id, control_id)
);
CREATE TABLE tm_control_components (
    control_id   uuid NOT NULL REFERENCES tm_controls (id) ON DELETE CASCADE,
    component_id uuid NOT NULL REFERENCES tm_components (id) ON DELETE CASCADE,
    position     integer NOT NULL,
    PRIMARY KEY (control_id, component_id)
);
CREATE TABLE security_test_threats (
    security_test_id uuid NOT NULL REFERENCES security_tests (id) ON DELETE CASCADE,
    threat_id        uuid NOT NULL REFERENCES tm_threats (id) ON DELETE CASCADE,
    position         integer NOT NULL,
    PRIMARY KEY (security_test_id, threat_id)
);
CREATE TABLE security_test_controls (
    security_test_id uuid NOT NULL REFERENCES security_tests (id) ON DELETE CASCADE,
    control_id       uuid NOT NULL REFERENCES tm_controls (id) ON DELETE CASCADE,
    position         integer NOT NULL,
    PRIMARY KEY (security_test_id, control_id)
);

CREATE INDEX tm_zones_tm_idx ON tm_zones (threat_model_id);
CREATE INDEX tm_assets_tm_idx ON tm_assets (threat_model_id);
CREATE INDEX tm_components_tm_idx ON tm_components (threat_model_id);
CREATE INDEX tm_flows_tm_idx ON tm_flows (threat_model_id);
CREATE INDEX tm_flows_source_idx ON tm_flows (source_id);
CREATE INDEX tm_flows_target_idx ON tm_flows (target_id);
CREATE INDEX tm_threats_tm_idx ON tm_threats (threat_model_id);
CREATE INDEX tm_controls_tm_idx ON tm_controls (threat_model_id);
CREATE INDEX security_tests_tm_idx ON security_tests (threat_model_id);
CREATE INDEX tm_component_assets_asset_idx ON tm_component_assets (asset_id);
CREATE INDEX tm_flow_assets_asset_idx ON tm_flow_assets (asset_id);
CREATE INDEX tm_threat_targets_component_idx ON tm_threat_targets (component_id);
CREATE INDEX tm_threat_targets_flow_idx ON tm_threat_targets (flow_id);
CREATE INDEX tm_threat_controls_control_idx ON tm_threat_controls (control_id);
CREATE INDEX security_test_threats_threat_idx ON security_test_threats (threat_id);
