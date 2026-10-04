-- Initial schema. See docs/specification/02-domain-model.md.

CREATE TABLE projects (
    id          uuid PRIMARY KEY,
    name        text NOT NULL,
    description text,
    version     integer NOT NULL DEFAULT 1,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX projects_page_idx ON projects (created_at, id);

CREATE TABLE components (
    id          uuid PRIMARY KEY,
    project_id  uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    name        text NOT NULL,
    description text,
    version     integer NOT NULL DEFAULT 1,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX components_project_idx ON components (project_id, created_at, id);

CREATE TABLE features (
    id                   uuid PRIMARY KEY,
    component_id         uuid NOT NULL REFERENCES components (id) ON DELETE CASCADE,
    name                 text NOT NULL,
    description          text,
    scenario_description text,
    tags                 text[] NOT NULL DEFAULT '{}',
    version              integer NOT NULL DEFAULT 1,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX features_component_idx ON features (component_id, created_at, id);

CREATE TABLE models (
    id          uuid PRIMARY KEY,
    feature_id  uuid NOT NULL REFERENCES features (id) ON DELETE CASCADE,
    name        text NOT NULL,
    description text,
    status      text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'ready')),
    variables   jsonb NOT NULL DEFAULT '[]',
    version     integer NOT NULL DEFAULT 1,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX models_feature_idx ON models (feature_id, created_at, id);

CREATE TABLE states (
    id          uuid PRIMARY KEY,
    model_id    uuid NOT NULL REFERENCES models (id) ON DELETE CASCADE,
    name        text NOT NULL,
    description text,
    kind        text NOT NULL CHECK (kind IN ('initial', 'normal', 'final')),
    pos_x       double precision,
    pos_y       double precision,
    sort_order  integer NOT NULL
);
CREATE INDEX states_model_idx ON states (model_id, sort_order);

CREATE TABLE transitions (
    id         uuid PRIMARY KEY,
    model_id   uuid NOT NULL REFERENCES models (id) ON DELETE CASCADE,
    from_state uuid NOT NULL REFERENCES states (id) ON DELETE CASCADE,
    to_state   uuid NOT NULL REFERENCES states (id) ON DELETE CASCADE,
    event      text NOT NULL,
    guard      text,
    action     text,
    expected   text,
    sort_order integer NOT NULL
);
CREATE INDEX transitions_model_idx ON transitions (model_id, sort_order);

-- Full snapshot per model version (FR-014).
CREATE TABLE model_versions (
    model_id   uuid NOT NULL REFERENCES models (id) ON DELETE CASCADE,
    version    integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    summary    text,
    snapshot   jsonb NOT NULL,
    PRIMARY KEY (model_id, version)
);

CREATE TABLE test_cases (
    id                      uuid PRIMARY KEY,
    feature_id              uuid NOT NULL REFERENCES features (id) ON DELETE CASCADE,
    name                    text NOT NULL,
    description             text,
    preconditions           text,
    priority                text CHECK (priority IN ('low', 'medium', 'high')),
    status                  text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'approved', 'deprecated')),
    tags                    text[] NOT NULL DEFAULT '{}',
    origin                  text NOT NULL CHECK (origin IN ('manual', 'generated', 'ai')),
    generated_from_model_id uuid REFERENCES models (id) ON DELETE SET NULL,
    steps                   jsonb NOT NULL DEFAULT '[]',
    version                 integer NOT NULL DEFAULT 1,
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX test_cases_feature_idx ON test_cases (feature_id, created_at, id);

-- Deleting a model, state or transition removes assignments, never test cases.
CREATE TABLE assignments (
    test_case_id  uuid NOT NULL REFERENCES test_cases (id) ON DELETE CASCADE,
    model_id      uuid NOT NULL REFERENCES models (id) ON DELETE CASCADE,
    state_id      uuid REFERENCES states (id) ON DELETE CASCADE,
    transition_id uuid REFERENCES transitions (id) ON DELETE CASCADE,
    step_order    integer CHECK (step_order >= 1),
    created_at    timestamptz NOT NULL DEFAULT now(),
    CHECK ((state_id IS NULL) <> (transition_id IS NULL)),
    UNIQUE NULLS NOT DISTINCT (test_case_id, state_id, transition_id, step_order)
);
CREATE INDEX assignments_model_idx ON assignments (model_id);
CREATE INDEX assignments_state_idx ON assignments (state_id) WHERE state_id IS NOT NULL;
CREATE INDEX assignments_transition_idx ON assignments (transition_id) WHERE transition_id IS NOT NULL;

CREATE TABLE proposals (
    id                  uuid PRIMARY KEY,
    project_id          uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    feature_id          uuid REFERENCES features (id) ON DELETE SET NULL,
    model_id            uuid REFERENCES models (id) ON DELETE SET NULL,
    kind                text NOT NULL,
    status              text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'rejected')),
    payload             jsonb NOT NULL,
    rationale           text,
    source              text,
    resulting_entity_id uuid,
    reject_reason       text,
    created_at          timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX proposals_project_idx ON proposals (project_id, created_at, id);

CREATE TABLE jobs (
    id           uuid PRIMARY KEY,
    status       text NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    proposal_ids uuid[] NOT NULL DEFAULT '{}',
    error        text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

-- Singleton row. API keys are never stored here (see docs/specification/07-ai-integration.md).
CREATE TABLE ai_settings (
    id                     boolean PRIMARY KEY DEFAULT true CHECK (id),
    provider               text NOT NULL DEFAULT 'none',
    base_url               text,
    model                  text,
    max_tokens_per_request integer
);
