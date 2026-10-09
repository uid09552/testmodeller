# Tasks

## 1. Contract and domain (requires approval)

- [x] 1.1 Get explicit approval of the API changes: `implementationUrl`/`backlogUrl` on `TestCase` and `TestCaseInput`, and `GET /projects/{projectId}/traceability`; then update `openapi.yaml` and `05-api.md` and verify the spec validates
- [x] 1.2 Add the requirement (FR) and fields to `03-requirements.md` and `02-domain-model.md`; verify they match the spec scenarios

## 2. Backend

- [x] 2.1 Add the migration: new columns, `backlog_key`, backfill from description lines with a down script; verify it on a fresh database and on a copy with legacy descriptions
- [x] 2.2 Add validation and normalisation of the URLs and the new fields in the storage and api crates; verify tests for valid, invalid and normalised-equal URLs
- [x] 2.3 Implement the traceability query with the gap rules, tenant and project scoped; verify integration tests for grouping, several test cases per item, untraced, unimplemented, linked-to-nothing and another tenant

## 3. Frontend

- [x] 3.1 Read and write the new fields in `model-mapping.ts`, keeping a fallback for old description lines; verify mapping tests for round trip and legacy data
- [x] 3.2 Build the Traceability page (matrix, filters, search, gaps tab, links open safely, route and nav entry); verify component tests
- [x] 3.3 Add CSV export with one row per backlog item and test case; verify the CSV content in a test

## 4. Docs and checks

- [x] 4.1 Describe the page and the meaning of "requirement" in `06-ui.md`; run `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `ng lint`, `ng test` and verify all pass
