---
title: 0003. AI chat panel runs on the existing proposal endpoints
type: adr
status: accepted
date: 2026-10-05
tags: [ai, api, ui]
related: [0001-tech-stack.md, ../specification/06-ui.md, ../specification/07-ai-integration.md]
---

# 0003. AI chat panel runs on the existing proposal endpoints

## Context

The model editor needs a chat panel where the user asks for scenarios and test
cases in free text and the agent answers by calling the backend.

A chat-shaped API (`POST /ai/chat` with a message history) would have been the
obvious fit, but the API contract is fixed and may not change without explicit
approval (AGENTS.md rule 6). The contract already carries everything a single
turn needs: `ProposalRequest` has a free-text `prompt`, a `kind`, a feature and
an optional model; the work runs as a job; proposals are fetched, accepted or
rejected individually.

## Decision

The chat panel is a client-side transcript over the existing endpoints. One
user message produces one `POST /ai/proposals` call; the panel polls
`GET /jobs/{id}`, loads each resulting proposal and renders it as a card in the
transcript. `kind` is inferred from the message unless the user picks it.

The transcript is frontend-only state. No conversation is persisted and no
message history is sent to the backend, so each turn stands alone; the context
the model sees is the feature and model context the backend assembles, exactly
as for any other proposal request.

Accepting a card calls `POST /proposals/{id}/accept` first and applies the
payload to the open model only after the call succeeds.

## Consequences

- No API change, and the panel inherits the backend's validation, token budget
  and prompt versioning.
- Follow-up questions ("make it shorter") do not work as a conversation; they
  are sent as a fresh instruction. A chat endpoint with history would need an
  approved contract change.
- The panel depends on backend state the local editor does not own: the feature
  must exist in the backend database and an AI provider must be configured.
  Both are reported in the transcript as errors rather than hidden.
- Proposal review stays human-in-the-loop, which is what FR-033 and AGENTS.md
  rule 8 require.
