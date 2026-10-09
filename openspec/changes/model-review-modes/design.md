# Design

## Context

The editor page hosts the canvas, the right column (Test Cases, Properties, AI) and the bottom panel (scenario, test cases, validation), all driven by `ModelEditorStore`. Editor tests are `StateTest`s on states. The mapping keeps neither their other assignments nor step orders, but `GET /models/{id}/test-cases` returns them. Generated tests now assign every step's transition (stale-test change). Guards and actions are evaluated only in Rust (`tm_domain::expr`: `check_guard`, `eval_guard`, `apply_action`, `initial_env`). The SPA never sees the token or role (FR-048). With `persist-canvas-layout`, the editor holds the model's `variables`. See proposal.md.

## Goals / Non-Goals

**Goals:**
- One expression implementation (Rust).
- Review features reuse the canvas, with no second renderer.

**Non-Goals:**
- Hiding editing from users without the Editor role. The SPA cannot know the role (FR-048). Present mode is a choice, not a permission; the server still enforces permissions.
- Editing variables (separate feature).
- Random or automatic simulation runs.

## Decisions

- **Simulation is a stateless backend call.** `POST /models/{modelId}/simulate` takes `{ graph: ModelInput, stateId?, env?, take?: transitionId }` and returns `{ stateId, env, final, transitions: [{ transitionId, enabled, reason? }] }`. Without `stateId` it starts at the initial state with `initial_env`. The graph in the body is the editor's current one, so unsaved edits are simulated. `modelId` scopes the call to the tenant, and the graph is not stored. Alternative: port the evaluator to TypeScript. Rejected: two implementations of the expression language would drift, and project rules keep domain logic in the backend. The cost is one request per step, which is acceptable for a human clicking.
- The step function lives in `tm_generation` next to the generator's `Machine::step`, so "same semantics" is literally the same code. A property test replays generated paths through it.
- **Simulation state in the frontend** is a stack of `{stateId, env, takenTransitionId}` frames. Step back pops a frame. "Save as test case" reuses `createTestCase` with transition and state assignments by step order, like `path_to_test_case`.
- **Present mode** is a page-level signal `mode: 'edit' | 'present' | 'table'` mirrored in the `view` query parameter. In present mode the canvas gets `readonly`: drag pans, and handles, connectors, context menu edit entries and delete keys are off.
- **Path highlight:** `model-mapping` keeps each test case's assignments (`{targetId, stepOrder}`) on `StateTest` as read-only data. The store has `highlight: { testId, steps: Map<elementId, number[]> } | null`. The canvas adds `--highlighted` and `--dimmed` classes: dimmed elements get reduced opacity plus dashed strokes, and highlighted ones get thicker strokes and numbered badges, so the difference is not colour alone. Cross-page "Show path" navigates to `/models/{id}?highlight={testCaseId}`, which the page resolves after load.
- **Table view** is a component bound to the store (same `updateNode`, `updateEdge`, `addEdge`, `removeNode` and undo). It uses a grid pattern with roving tabindex: arrow keys move between cells, Enter edits, Esc cancels, and Delete on a row asks for confirmation.

## Risks / Trade-offs

- [Simulation request carries the whole graph each step] → Models are small (a 200-state model is about 100 KB). The body limit is 16 MB. Debounced clicking is not needed.
- [Present mode looks like a permission] → It is labelled "Present" (not "Read-only access"), and the docs say the server enforces roles.
- [Highlight data for per-state editor tests is trivial] → They highlight one state. Generated tests are where it matters.
- [Table view and canvas diverge in features] → Both call the same store operations. The table's cells come from the same field definitions as the Properties panel.

## Migration Plan

No data migration. The new endpoint is additive.
