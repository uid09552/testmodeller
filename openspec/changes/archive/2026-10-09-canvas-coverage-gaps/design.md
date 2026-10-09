# Design

## Context

Editor tests are held per state (`node.tests`) and travel to the backend as test cases assigned to states on save. Generated test cases assign to transitions through the API. The server's `Coverage` response lists `uncoveredStateIds` and `uncoveredTransitionIds`. The editor store already has a `refresh`-style load from the backend and a dirty flag. See proposal.md.

## Goals / Non-Goals

**Goals:**
- Gaps visible on the canvas with no API change.
- States update live; transitions are honest about being from the last save.

**Non-Goals:**
- Choosing pair or path criteria (the endpoint does not take one).
- The Coverage dashboard page.
- Colouring by test results (see `test-results-import`).

## Decisions

- **State coverage computed client-side** from `node.tests.length > 0`, same definition as the chips. Alternative: always use the server value — rejected, it would lag behind edits.
- **Transition coverage from the endpoint**, fetched when the overlay is on and after each save, held in the store as `{ uncoveredTransitionIds, asOfVersion }`. Alternative: assign tests to transitions in the editor — a much larger change.
- **Overlay state is a store signal** (`coverageView: 'off' | 'states' | 'transitions' | 'both'`), local to the editor, not persisted with the model.
- **Rendering:** uncovered elements get a dashed amber outline and a small "0" marker, so the cue survives greyscale and colour blindness; transitions get the same dashed style and an accessible label.
- Clicking an uncovered state reuses the existing double-click-on-chips behaviour (select, open Test Cases tab).

## Risks / Trade-offs

- [Transition data stale after edits] → labelled "as of last save"; refetched on save.
- [Extra request per save] → only when the overlay is on.
- [Mismatch: a state with only an unsaved test counts covered locally but not on the server] → states use local data only, so counts stay self-consistent.
