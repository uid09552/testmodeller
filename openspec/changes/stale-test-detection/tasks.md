# Tasks

## 1. API contract (requires approval)

- [ ] 1.1 Get explicit approval for the new `GET /models/{modelId}/stale-tests` endpoint and its response schema; then update `openapi.yaml` and `05-api.md` and verify the spec validates

## 2. Backend

- [ ] 2.1 Implement the path check in the domain/generation crate; verify unit tests for each reason (unassigned step, disconnected, not from initial, unsatisfiable guard), a healthy path, and manual/AI tests being skipped
- [ ] 2.2 Add the route in the api crate with tenant scoping; verify integration tests for results, 404 on another tenant's model, and no data being modified
- [ ] 2.3 Update `04-test-generation.md`; verify it states the reasons and the limits

## 3. Frontend

- [ ] 3.1 Add the call and store state (fetch on open, after save, after generation); verify store tests
- [ ] 3.2 Mark stale tests with a reason in the Test Cases tab, the Test Cases page and on the canvas chips, and list them as warnings in Validation; verify component tests including the non-colour marker
- [ ] 3.3 Describe the behavior in `docs/specification/06-ui.md`; run `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `ng lint`, `ng test` and verify all pass
