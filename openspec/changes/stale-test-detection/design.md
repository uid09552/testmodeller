# Design

## Context

A generated test case stores `steps[]`, `generatedFromModelId` and `assignments[]` where each assignment names a model, a state or transition and optionally a `stepOrder`. Generation walks the model graph with guards using `tm_domain` graph and expression code. Deleting a model element removes the assignment but keeps the test case (domain invariant). The editor holds a different shape: per-state tests assigned to a state, with no path; those are not path tests. See proposal.md.

## Goals / Non-Goals

**Goals:**
- Derive staleness on demand from current data; deterministic and cheap enough to call when a model is opened or saved.

**Non-Goals:**
- Storing a "generated from version" on test cases or an automatic regeneration.
- Staleness of manual tests, which have no path.
- Fixing stale tests.

## Decisions

- **A pure function in `domain`/`generation`**: `check_path(model_graph, test_case) -> Vec<StaleReason>`. It rebuilds the transition sequence from assignments ordered by `stepOrder`, then checks: every step has a transition; consecutive transitions chain (`to` of one is `from` of the next); the first starts at the initial state; guards are satisfiable along the path using the existing evaluator. Alternative: store the generating model version and compare — rejected: it flags tests for any edit, however unrelated, and needs a migration.
- **One read-only endpoint** `GET /models/{modelId}/stale-tests` returning `[{ testCaseId, reasons: [{ code, stepOrder?, message }] }]`. Alternative: add a `stale` field to every `TestCase` — rejected: the answer depends on a model, and a test case can be assigned across models of its feature.
- **Only `origin = generated` tests** are checked, as they alone have a path by construction.
- **Frontend** fetches on model open, after save and after generation; the result is held in the store and used by chips, lists and the Validation tab.
- Reason codes are stable strings (`STEP_UNASSIGNED`, `STEPS_DISCONNECTED`, `NOT_FROM_INITIAL`, `GUARD_UNSATISFIABLE`) so the UI maps them to text and tests can assert on them.

## Risks / Trade-offs

- [Guard satisfiability is bounded and may report false "unsatisfiable"] → reuse the generator's own limits and mark the reason "could not be proven satisfiable" when the search cap is hit.
- [Large models] → check only generated tests, one pass over assignments, within NFR-002 latency; measure on a 200-state model.
- [Per-state editor tests are not paths] → excluded; documented in the spec so no one expects them flagged.
