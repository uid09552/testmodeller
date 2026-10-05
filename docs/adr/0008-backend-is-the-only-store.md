---
title: 0008. The backend is the only store of models
type: adr
status: accepted
date: 2026-10-05
tags: [frontend, api, persistence]
related: [0006-ai-context-sync.md, ../specification/05-api.md]
---

# 0008. The backend is the only store of models

## Context

The projects tree and model content lived in `localStorage`, with the backend
holding only a partial copy made on demand for the AI panel (ADR 0006). Deleting
everything and reloading brought demo data back, and a second browser saw
nothing.

## Decision

The frontend keeps no model data in the browser. `ExplorerStore` reads projects,
components, features and models from the API on start and changes them through
it. `ModelPersistenceService` loads the open model from the API and saves it
there, debounced, with `If-Match` versions; a 412 stops autosave until the user
picks their version or the other one. The test case list reads through the API.
New models are created in the backend first, then opened by their backend id.
`ModelSyncService` and the demo data are gone.

## Consequences

- The contract does not carry shapes, colours, sizes, groups, edge curves, the
  `decision` kind or the `review` status, so they are lost on reload. Keeping
  them needs a contract change (for example a presentation field on the model),
  which requires approval.
- There is no offline mode: edits made while the server is unreachable are
  retried but lost if the tab is closed.
- Starting the tree costs one request per project, component and feature; a
  tree endpoint with counts would reduce that.
