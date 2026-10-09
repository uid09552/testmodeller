# Design

## Context

ADR 0008 made the backend the only store: `ModelPersistenceService` autosaves through `PUT /models/{id}` (atomic graph replace, `If-Match`), and `model-mapping.ts` converts between the canvas and the contract. What the contract cannot hold is either kept in memory by `fromRemote`'s merge or lost. `remoteFingerprint` deliberately ignores presentation, so a colour change sends nothing. Every successful `PUT` writes a snapshot to `model_versions`. Duplicating a model remaps all element ids (`remap_ids`). `ModelInput.variables` defaults to an empty list on the server, and the frontend never sends it. See proposal.md.

## Goals / Non-Goals

**Goals:**
- One opaque, versioned layout document per model, saved with the graph in the same request.
- The editor round-trips everything it loads.

**Non-Goals:**
- Editing variables in the editor (a separate feature, see `feature.md`).
- The three-way model status (`review`), which is a contract question, not layout.
- Per-user view state (zoom, pan, selection): that stays in the browser.

## Decisions

- **`layout` is a free-form JSON object on `Model` and `ModelInput`**, stored in `models.layout jsonb`. The backend checks only that it is an object of at most 256 KB. Typed per-element fields were rejected: every visual feature would need another contract change, and the backend has no use for them.
- **Format, owned by the frontend and versioned with `"v": 1`:**
  ```json
  {
    "v": 1,
    "states":      { "<stateId>": { "shape": "diamond", "color": "#4f6ef2", "w": 172, "h": 104, "decision": true } },
    "transitions": { "<transitionId>": { "curve": 46, "fromAnchor": "right", "toAnchor": "left" } },
    "groups":      [ { "id": "g1", "label": "Auth", "color": "#4f6ef2", "opacity": 0.1, "x": 0, "y": 0, "w": 300, "h": 200 } ],
    "testSeq":     { "<testCaseId>": 3 },
    "nextTestSeq": 5
  }
  ```
  Only values that differ from the defaults are written. Later changes (`edge-routing`, `canvas-navigation`) add keys inside the same entries. An unknown `v` is read as far as it is understood.
- **Saved with the graph, one request.** `toModelInput` includes `layout`, and `remoteFingerprint` includes it, so a layout-only edit is debounced and saved like a move. A separate `PUT /models/{id}/layout` without a version bump was considered: it would avoid versions for cosmetic edits, but it adds an endpoint and a second concurrency path, and position changes already create versions.
- **Element references stay ids.** On duplicate, the backend replaces every occurrence of an old element id in the layout (object keys and string values) with the new id. This stays format-agnostic: it does not need to know which keys hold ids. JSON export carries `layout` verbatim; import remaps it with the same id map it already uses.
- **Snapshots include `layout`**, so restoring a version restores its layout.
- **Variables round-trip:** `PersistedModel` gains `variables`, filled from `GET /models/{id}` and sent back unchanged in every `ModelInput`.
- **`decision` kind:** the contract keeps `normal`, and the layout's `decision: true` restores the editor kind.

## Risks / Trade-offs

- [Version history fills with cosmetic versions] → Saves are already debounced (2 s). The version list shows the summary, and a later change could mark layout-only versions.
- [Two editors overwrite each other's layout] → Same `If-Match` as the graph: the second gets a conflict, as today.
- [Layout references a state id that is reused] → Ids are UUIDs (non-UUID local ids are rewritten by `withUuids` before save).
- [An old frontend saves a model without `layout`] → An absent `layout` in a `PUT` keeps the stored one. Only an explicit `null` clears it.
- [Data loss already happened for variables] → Not recoverable from the model itself. Earlier versions in `model_versions` still hold them, so restoring one brings them back.

## Migration Plan

Migration `0007`: `ALTER TABLE models ADD COLUMN layout jsonb`. Existing models get no layout and show defaults, which is today's behaviour. Rollback: drop the column. The frontend tolerates a missing `layout` either way.
