-- Feature test cases that cover threats (ADR 0016), and the feature test case
-- a security test was converted into.
CREATE TABLE test_case_threats (
    test_case_id uuid NOT NULL REFERENCES test_cases (id) ON DELETE CASCADE,
    threat_id    uuid NOT NULL REFERENCES tm_threats (id) ON DELETE CASCADE,
    created_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (test_case_id, threat_id)
);
CREATE INDEX test_case_threats_threat_idx ON test_case_threats (threat_id);

ALTER TABLE security_tests
    ADD COLUMN converted_to_test_case_id uuid REFERENCES test_cases (id) ON DELETE SET NULL;
CREATE INDEX security_tests_converted_idx ON security_tests (converted_to_test_case_id);
