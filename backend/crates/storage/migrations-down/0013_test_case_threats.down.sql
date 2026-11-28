DROP INDEX IF EXISTS security_tests_converted_idx;
ALTER TABLE security_tests DROP COLUMN IF EXISTS converted_to_test_case_id;
DROP TABLE IF EXISTS test_case_threats;
