# Tasks

## 1. Contract (requires approval)

- [x] 1.1 Get explicit approval for `POST /models/{modelId}/simulate` (request: graph, optional stateId, env, take; response: stateId, env, final, transitions with enabled and reason); then update `openapi.yaml`, `05-api.md` and `04-test-generation.md`; verify the spec parses and Redocly shows no new problem kinds

## 2. Simulation backend

- [x] 2.1 Add a one-step simulation function in `tm_generation` sharing the generator's guard and action evaluation; verify unit tests for start, enabled and blocked transitions with reasons, an action applied, a dead end and a final state, plus a property test that every generated path replays step by step
- [x] 2.2 Add the route with tenant scoping and graph validation (no initial state, several initial states, bad expressions give 422 with reasons); verify API tests, including 404 for another tenant's model and that nothing is stored

## 3. Review in the editor

- [x] 3.1 Keep each test case's assignments and step orders in `model-mapping.ts` (read-only on `StateTest`); verify mapping tests
- [x] 3.2 Path highlight: store state, canvas classes and step badges, status message, Esc to clear, `?highlight=` on the model route, "Show path" in the Test Cases tab, the Test Cases page and the Traceability matrix; verify component tests including the non-colour cue and the repeated-transition numbering
- [x] 3.3 Present mode: page mode signal with `?view=present`, read-only canvas (drag pans; handles, connectors, edit menu entries and delete keys off), Esc / Edit to leave; verify component tests that no edit is possible and navigation still works

## 4. Simulation panel

- [x] 4.1 Simulation panel: start, enabled and blocked list with reasons, canvas marks for current state and available transitions, take, step back, restart, variable values, and a "stuck" message; verify component tests with the API mocked
- [x] 4.2 Save the trail as a manual test case assigned by step order; verify a component test of the created input

## 5. Table view

- [x] 5.1 Table view component (states and transitions tables) bound to the store, with cell editing, add and delete rows, shared undo, and validation issues per row; verify component tests for editing, adding a transition, deleting a state and undo
- [x] 5.2 Grid keyboard model (roving tabindex, arrows, Enter, Esc, Delete with confirmation), focus ring and accessible names; verify keyboard-only component tests

## 6. Docs and checks

- [x] 6.1 Document present mode, path highlight, simulation and table view in `06-ui.md`, and mark NFR-004's table view as delivered in `03-requirements.md`; run `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `ng lint`, `ng test`, `ng build` and verify all pass
