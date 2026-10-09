# Proposal

## Why

The editor is built for authoring. Reviewing a model with people who should not change it, checking what a generated test actually walks through, or sanity-checking guards before generating all require reading the diagram by eye. Keyboard and screen-reader users have no alternative to the canvas at all, although NFR-004 promises one (`feature.md`: the table view is specified but not built). Stale-test detection now assigns every step of a generated test, so a test's path can be shown exactly.

## What Changes

- **Present mode:** a read-only view of a model with the editing tools, palette and panels hidden. Navigation, search and highlights still work. It is a toggle in the editor and a shareable link (`/models/{id}?view=present`).
- **Test path highlight:** choosing "Show path" on a test case (Test Cases tab, Test Cases page, Traceability matrix) highlights its states and transitions on the canvas and numbers its steps in order. Everything else is dimmed. Esc clears it.
- **Simulation:** a step-through panel starts at the initial state with the variables' initial values, lists the transitions that are enabled or blocked (with the guard that blocks them), and takes one on click. It shows the variables and the trail of steps taken, can step back, and can save the trail as a manual test case. It always simulates what is on screen, including unsaved edits.
- **Table view (NFR-004):** an alternative to the canvas with a states table and a transitions table. Every cell can be edited, rows can be added and deleted, and it is fully keyboard operable. It edits the same model as the canvas, with the same undo.
- **API change (needs your approval before it is applied):** `POST /models/{modelId}/simulate` evaluates one simulation step for the graph the editor sends. The backend's expression language stays the only implementation.

## Capabilities

### New Capabilities
- `model-review`: Viewing a model without editing it, and highlighting the path of a test case on it.
- `model-simulation`: Stepping through a model's behaviour interactively, with guards and actions evaluated.
- `model-table-view`: Viewing and editing a model's states and transitions as tables, as the keyboard-accessible alternative to the canvas.

### Modified Capabilities

## Impact

- Backend: `api` (simulate route, DTOs), `domain`/`generation` (one-step evaluation reusing `tm_domain::expr`); `openapi.yaml`, `05-api.md`, `04-test-generation.md`.
- Frontend: model editor page (mode switch, present route parameter), canvas (highlight and dim layer), new simulation panel and table view components, test lists ("Show path" actions), `model-mapping.ts` (keep each test's assignments for the highlight).
- `docs/specification/06-ui.md`, `03-requirements.md` (NFR-004 status).
- Depends on `persist-canvas-layout` (variables reach the editor). Uses `canvas-navigation` search and reveal when present, but does not need it.
