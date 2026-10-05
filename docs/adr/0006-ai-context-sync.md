---
title: 0006. The AI panel syncs its own context to the backend
type: adr
status: superseded
date: 2026-10-05
tags: [ai, api, frontend]
related: [0003-ai-chat-on-proposal-endpoints.md, ../specification/07-ai-integration.md]
---

# 0006. The AI panel syncs its own context to the backend

## Context

The editor keeps model content in the browser, so its ids mean nothing to the
backend. The AI endpoints are scoped to a feature — `ProposalRequest` requires
a `featureId`, and `states-and-transitions` also needs a `modelId` — so every
question from the chat panel failed with "feature not found".

Three ways out: move the whole editor onto the API first; relax the contract so
proposals can be requested without stored entities; or create the entities the
request needs, on demand.

## Decision

The panel creates them on demand. Before the first request of a turn,
`ModelSyncService` walks the path the tree gives it — project, component,
feature — creating anything that has no backend id yet, then creates or
replaces the model with the canvas's current graph. Local id to backend id
mappings are remembered in the browser, so each thing is created once.

The graph is pushed on **every** request, not only the first, so the assistant
reasons about what is on the canvas now.

Canvas content is translated as the contract models it: states get fresh UUIDs
because transitions reference them by id, `decision` becomes `normal` (the
contract has three kinds), and an edge pointing at a state that is not in the
graph is dropped rather than failing the sync as structurally broken.

## Consequences

- The assistant works against a locally created model, which was the whole
  point. No contract change was needed.
- Remembered ids can outlive the backend's data — `make dev` starts a fresh
  database on every run. A 404 anywhere in the sync therefore forgets that
  model's whole path and rebuilds it, once; a second 404 is reported rather
  than retried.
- The sync is one-way and partial: the backend gets a copy of the graph, and
  the browser remains the editor's source of truth. Accepted proposals are
  applied to the browser's copy, and the backend's copy catches up on the next
  question.
- Entities accumulate in the backend for models that were only ever asked
  about. They are real projects in the caller's tenant, so they are visible and
  deletable through the API.
- Relaxing the contract was the alternative. It would have avoided the writes,
  but a proposal that cannot be accepted into anything is not much use, and
  `POST /proposals/{id}/accept` creates entities under a feature regardless.
- This is a bridge, not the destination. Moving the editor onto the API makes
  the service unnecessary, and it should be deleted then rather than kept.
