---
title: 0007. The AI provider is configured in the environment
type: adr
status: accepted
date: 2026-10-05
tags: [ai, configuration, deployment]
related: [../specification/07-ai-integration.md, 0002-container-deployment.md]
---

# 0007. The AI provider is configured in the environment

## Context

The provider — Claude, Ollama or an OpenAI-compatible endpoint — its model,
endpoint and key were set in a Settings form and stored by the backend. That
made the provider part of the application's data rather than of its
deployment: it could not be set before first start, could not be reviewed
alongside the rest of the stack's configuration, and differed between
environments in a way nothing recorded.

## Decision

The provider is configured with `TM_AI_*` variables in `.env`, which
`compose.yaml` passes to the backend. The backend validates them at startup and
writes them over the stored settings, so `GET /settings/ai` reports what is in
use. Settings becomes read-only: it shows the effective configuration and the
variables to change.

Product names (`claude`, `ollama`) are accepted alongside the contract's
protocol names (`anthropic`, `local`), because they are what people write in a
`.env` file.

## Consequences

- One place configures the stack. Switching provider is an edit and a restart,
  reviewable like any other configuration change.
- A typo stops the server with a message naming the variable, instead of
  failing the first proposal request.
- `PUT /settings/ai` still exists, because the contract defines it, but a change
  made through it lasts only until the next restart. Rejecting it while the
  environment is in charge would need a contract change (a `409`), which has
  not been agreed.
- The encrypted key storage added for the form remains, for keys saved through
  that endpoint; with the key in `TM_AI_API_KEY` it is not used.
