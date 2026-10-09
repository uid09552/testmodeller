---
title: 0010. The editor's layout is stored with the model, opaque to the backend
type: adr
status: accepted
date: 2026-10-09
tags: [frontend, api, persistence]
related: [0008-backend-is-the-only-store.md, ../specification/05-api.md]
---

# 0010. The editor's layout is stored with the model, opaque to the backend

## Context

ADR 0008 made the backend the only store, but the contract only held the
graph. Shapes, colours, sizes, groups and transition curves lived in browser
memory and were lost on every reload, and new canvas features (bending
transitions, collapsing groups) would be lost the same way. The editor also
never sent `variables`, so its saves deleted them.

## Decision

- `Model` and `ModelInput` carry an optional `layout` JSON object. The
  frontend owns its format and versions it with a `v` key. The backend checks
  only that it is an object of at most 256 KB, and otherwise stores and
  returns it without interpreting it.
- It is saved with the graph in the same `PUT`, under the same `If-Match`. An
  absent `layout` keeps the stored one, and `null` clears it.
- It is part of version snapshots, duplicates and the JSON export and import.
  On duplicate and import, the backend replaces every occurrence of an old
  element id inside it with the new id, without knowing which keys hold ids.
- The editor sends back the model's `variables` unchanged.

## Consequences

- Layout-only edits create model versions, as position changes already did.
- New presentation features need no contract change.
- The backend cannot validate layout content. A bad layout can only break the
  editor's rendering, and the editor falls back to defaults for anything it
  does not understand.
