-- Imported execution results of test cases (FR-050..FR-053).
-- See docs/specification/02-domain-model.md (TestResult).
--
-- One row per test case and run: re-importing the same file adds nothing.
-- The tenant is copied from the project at import time, so a result can be
-- checked against the caller without walking up the tree.

CREATE TABLE test_results (
    id           uuid PRIMARY KEY,
    tenant_id    text NOT NULL,
    test_case_id uuid NOT NULL REFERENCES test_cases (id) ON DELETE CASCADE,
    run_id       text NOT NULL,
    status       text NOT NULL CHECK (status IN ('passed', 'failed', 'skipped', 'error')),
    duration_ms  bigint CHECK (duration_ms >= 0),
    message      text,
    executed_at  timestamptz NOT NULL,
    source       text NOT NULL,
    imported_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (test_case_id, run_id)
);

-- Latest result per test case: newest run first, ties by import time.
CREATE INDEX test_results_latest_idx ON test_results (test_case_id, executed_at DESC, imported_at DESC);
