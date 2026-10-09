# Proposal

## Why

Generated test cases describe a path through a model. When the model changes, those tests can silently stop matching it: an element is deleted, a transition is rewired, or a guard becomes unsatisfiable. Regenerate-and-diff (FR-023) only helps someone who remembers to regenerate. Deleting a state or transition removes its assignments but keeps the test case, so orphaned tests look healthy.

## What Changes

- The backend works out, for a model, which of its generated test cases are stale and why.
- The editor and the test case lists flag stale tests with their reason, and a model-level count appears in the Test Cases tab and the Validation tab.
- Stale tests are never changed or deleted automatically; the user decides to regenerate, edit or dismiss.
- **API change (needs your approval before it is applied):** one new read-only endpoint, `GET /models/{modelId}/stale-tests`.

## Capabilities

### New Capabilities
- `test-staleness`: Detecting and showing generated test cases that no longer match their model.

### Modified Capabilities

## Impact

- `backend/crates/generation` or `domain` (path re-check using the existing graph and expression code), `backend/crates/api` (new route), `storage` (read assignments).
- `docs/specification/openapi.yaml`, `05-api.md`, `04-test-generation.md`, `06-ui.md`.
- Frontend: test lists and editor chips.
- No schema migration: staleness is derived from current data, not stored.
