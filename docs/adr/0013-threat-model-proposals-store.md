---
title: 0013. Threat model proposals have their own store and share the job mechanism
type: adr
status: accepted
date: 2026-10-17
tags: [ai, threat-modeling, api]
related: [../specification/07-ai-integration.md, 0003-ai-chat-on-proposal-endpoints.md, 0007-ai-provider-from-environment.md, 0011-threat-model-entities-and-security-tests.md]
---

# 0013. Threat model proposals have their own store and share the job mechanism

## Context

The model editor's assistant stores proposals in `proposals` (project, feature, model; payloads for models, states and test cases) and runs them as jobs. The threat model needs proposals for different entities, scoped to a threat model.

## Decision

- Threat model proposals are rows in a new table `tm_proposals` owned by the threat model. The existing `proposals` table is unchanged.
- The job mechanism (`jobs`, `GET /jobs/{id}`), the provider adapters, the `TM_AI_*` configuration, the retry-with-error and the prompt versioning are shared. The `ai` crate gains a `tm` module.
- Proposal routes are nested under `/threat-models/{id}/ai/proposals`, so they need no new tenant scope (ADR 0011).
- Unknown fields in payloads are rejected by the parser, so AI cannot set statuses, treatments, acceptances, evidence or thresholds.
- AI assistance is opt-in per threat model.
- No context sync is needed (contrast ADR 0006): the threat model already lives in the backend.

## Consequences

- Two proposal tables exist. A shared proposal store remains possible later; payloads carry their kind.
- Accepting a proposal is the user's confirmation, so AI-created threats start open (AGENTS.md rule 8 is met by the pending staging).
- Conversations are not persisted or sent as history (as ADR 0003).
