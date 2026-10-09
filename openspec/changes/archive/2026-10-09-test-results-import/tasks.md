# Tasks

## 1. Contract and domain (requires approval)

- [x] 1.1 Get explicit approval of the API changes: import endpoint, history endpoint, `TestCase.lastResult`, `CoverageCount.passing`; then update `openapi.yaml` and `05-api.md` and verify the spec validates
- [x] 1.2 Add a requirement (FR) and the `TestResult` entity to `03-requirements.md` and `02-domain-model.md`; verify they match the spec scenarios

## 2. Backend

- [x] 2.1 Add migration `0005` for `test_results` with the unique and lookup indexes; verify it applies on a fresh and an existing database
- [x] 2.2 Implement JUnit and Cucumber parsers with size and entity limits; verify unit tests with real sample files, malformed input and an XML entity-expansion payload
- [x] 2.3 Implement matching (tag, then unique name), idempotent storage and the report, tenant-scoped; verify integration tests for match by tag, by name, ambiguous, unknown, repeat import and another tenant's tag
- [x] 2.4 Emit the `@tm-<uuid>` tag in Gherkin and a `tmId` CSV column; verify export tests
- [x] 2.5 Add `lastResult` to test cases and `passing` to coverage; verify API tests

## 3. Frontend

- [x] 3.1 Add an import dialog (file, format, dry-run preview, report); verify component tests including error display
- [x] 3.2 Show the latest result as a labelled badge in lists and on chips, and "passing" in the coverage figures; verify tests including a non-colour cue
- [x] 3.3 Document the feature in `06-ui.md` and how to carry the tag in JUnit and Cucumber; run `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `ng lint`, `ng test` and verify all pass
