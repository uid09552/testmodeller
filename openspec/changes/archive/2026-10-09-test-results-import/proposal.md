# Proposal

## Why

Coverage today means "modelled": a test case exists for a state or transition. It says nothing about whether the test passes. Teams run their tests elsewhere (CI, Playwright, Cucumber), so the tool cannot execute them (out of scope for v1), but it can read the results and show whether covered behavior is actually passing.

## What Changes

- Import JUnit XML and Cucumber JSON results for a project; each result is matched to a test case.
- Store each run's result per test case (status, time, message), keeping history.
- Show the latest result on test cases, state chips and lists, and count "passing" in coverage.
- Make matching reliable: the Gherkin and CSV exports carry a stable test case tag, and the importer matches on it, falling back to the test case name within a feature.
- **API changes (need your approval before they are applied):** an import endpoint, a results read endpoint, and optional result fields on `TestCase` and `CoverageCount`. A new database table is required.

## Capabilities

### New Capabilities
- `test-results`: Importing, storing, matching and showing execution results of test cases.

### Modified Capabilities

## Impact

- `backend/crates/storage` (migration `0005`, new `test_results` table, tenant-scoped), `api` (routes, XML/JSON parsing with size and entity limits), `domain` (result entity).
- Export (`export.rs`, Gherkin tag and CSV column), `openapi.yaml`, `05-api.md`, `02-domain-model.md`, `03-requirements.md` (new FR), `06-ui.md`.
- Frontend: import dialog, result badges, coverage "passing".
- Supersedes nothing; test execution stays out of scope.
