# Proposal

## Why

The canvas holds a lot that the contract cannot express: shapes, colours, sizes, groups, transition curves and connector points, the `decision` kind and the display numbers of test cases. None of it is stored (ADR 0008, `model-mapping.ts`), so a reload throws away every bit of layout work, and any new layout feature (bending arrows, label placement) would be lost the same way. Worse, the editor never sends `variables`, and a model save replaces the whole graph, so **every editor autosave deletes the variables** of a model that was created through the API, by import or by AI.

## What Changes

- **API change (needs your approval before it is applied):** an optional `layout` object on `Model` and `ModelInput`. The backend stores it and returns it, never interprets it, and caps its size.
- The editor writes its presentation into `layout` and restores it on load: state shape, colour and size, the `decision` kind, groups, transition curves and connector points, and test display numbers. Entries for deleted elements are dropped on save.
- Layout travels with the model: it is part of version snapshots (restore brings it back), of duplicating a model, and of the JSON export and import.
- The editor carries the model's `variables` through a save unchanged (it has no variable editing yet), fixing the silent deletion.
- Generation, validation, coverage, staleness and every other behaviour ignore `layout`.

## Capabilities

### New Capabilities
- `model-editor-persistence`: What the model editor stores and restores: the model graph it edits, the parts of the model it does not edit, and its own layout.

### Modified Capabilities

## Impact

- `docs/specification/openapi.yaml`, `05-api.md`, `02-domain-model.md`, `06-ui.md`; ADR 0008 gets a follow-up ADR.
- `backend/crates/storage` (migration: `models.layout jsonb`; versions, duplicate, export/import), `api` (DTOs, size check), `domain` (model meta).
- `frontend/src/features/models/state/model-mapping.ts`, `model-persistence.ts` (fingerprint now includes layout, so layout-only edits are saved), `core/api/org-api.ts`.
- Prerequisite for `edge-routing`, `canvas-navigation` and `model-review-modes`, which add layout data of their own.
- Layout-only edits now create a model version on save, as position changes already do.
