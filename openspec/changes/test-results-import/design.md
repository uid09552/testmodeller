# Design

## Context

There is no result data anywhere. Test cases are `TestCase` records with steps and assignments; the editor's links to implementation and backlog are stored as labelled lines in `description` (see `traceability-matrix`). Exports (`export.rs`) produce JSON, CSV and Gherkin; the Gherkin tags today are per-model refs like `LoginFlow_3`, which are not unique across a project. Migrations are numbered SQL files (`0001`–`0004`); all data is tenant-scoped. See proposal.md.

## Goals / Non-Goals

**Goals:**
- A reliable match from an external result to a test case.
- A small, queryable store of results with history.

**Non-Goals:**
- Running tests, scheduling, or live CI integration (a later CLI could post to the same endpoint).
- Attachments, logs, screenshots.
- Other formats (TRX, TAP, Allure) — the parser sits behind a format enum so they can be added.

## Decisions

- **Match key: a stable tag `@tm-<test case UUID>`** in Gherkin exports and a `tmId` CSV column, with exact-name fallback. Names are not stable or unique, and per-model refs collide across models, so they are poor keys. Alternative: require users to name tests with the UUID — rejected, users would never do it. The tag goes in Gherkin so Cucumber runners report it in `tags`; JUnit authors put it in the test name or a property.
- **Table `test_results`**: `id`, `tenant_id`, `test_case_id` (FK, cascade on delete), `run_id`, `status` (`passed|failed|skipped|error`), `duration_ms`, `message`, `executed_at`, `source` (file name, format). Unique on `(test_case_id, run_id)` for idempotent re-imports; index on `(test_case_id, executed_at desc)`.
- **`run_id`** is taken from the file (JUnit `timestamp` plus suite name, Cucumber start time) and hashed with the file content when absent, so re-importing the same file is idempotent.
- **Import endpoint** `POST /projects/{projectId}/test-results` with `format` (`junit|cucumber`) and the file as the request body (not multipart), 10 MB limit. Returns the report. Dry-run flag `?dryRun=true` previews matching.
- **Read endpoint** `GET /test-cases/{testCaseId}/results?limit=` for history. `TestCase` gains an optional read-only `lastResult`, and `CoverageCount` an optional `passing`. Alternative: a separate results table call per test case from the UI — rejected: lists would make N calls.
- **Parsing safety:** XML parsed with DTDs and external entities disabled; element and size limits; no file content is logged.
- **Passing coverage:** an element is passing when it has at least one covered test and every covered test's latest result is `passed`.

## Risks / Trade-offs

- [Runners drop the tag] → name fallback plus a clear unmatched report, and the docs show how to carry the tag for JUnit and Cucumber.
- [Table growth] → keep all results but expose only the latest in lists; a retention setting is a later decision.
- [Clock skew between runners] → latest is by `executed_at`, ties by import time.
- [Old exports use the previous tags] → they still import by name; nothing is lost.
