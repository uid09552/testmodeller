# Proposal

## Why

The canvas shows test chips on states that have tests, but says nothing about the states and transitions that have none. Spotting a gap means counting chips by eye. The server already knows which elements are uncovered (`GET /models/{id}/coverage` returns `uncoveredStateIds` and `uncoveredTransitionIds`), but the editor does not use it.

## What Changes

- A "Coverage" toggle in the canvas toolbar highlights uncovered states and transitions, and shows covered/total counts.
- A choice of what counts: states, transitions, or both. These are the only criteria the coverage data supports.
- States are computed live in the editor from their tests; transitions come from the server's coverage for the saved model and are marked as such until the next save.
- Clicking an uncovered element selects it and offers "Add test case" (states).
- No API change: the existing coverage endpoint is enough.

## Capabilities

### New Capabilities
- `coverage-gaps`: How the model editor shows which states and transitions lack test coverage.

### Modified Capabilities

## Impact

- `frontend/src/features/models/` (canvas, store, toolbar), `docs/specification/06-ui.md`.
- Uses the existing `GET /models/{modelId}/coverage`; `openapi.yaml` unchanged.
- Related but out of scope: the Coverage dashboard page still shows placeholder data (`features/coverage/coverage-dashboard.ts`); wiring it up is a separate change.
