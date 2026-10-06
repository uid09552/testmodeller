---
title: Requirements
type: specification
status: draft
tags: [requirements, functional, non-functional]
related: [00-overview.md, 05-api.md, 06-ui.md, 07-ai-integration.md]
---

# Requirements

## Functional

### Organization
- FR-001 Create, rename, delete components within a project.
- FR-002 Create, rename, move features between components.
- FR-003 Create models under a feature; edit the feature's scenario description.
- FR-004 Browse and filter by component, feature, tag, status.
- FR-005 Create, edit, delete test cases manually within a feature.
- FR-006 Assign test cases to model states and transitions (steps); view assigned test cases per state/transition.
- FR-007 Export and import project data.

### Model editing
- FR-010 Graphical editor to add/move/delete states and transitions.
- FR-011 Set event, guard, action and expected result on transitions.
- FR-012 Validate model live (single initial state, reachability, dead ends) and show issues.
- FR-013 Undo/redo in the editor.
- FR-014 Version history of models with restore.

### Test generation
- FR-020 Generate test cases for a model by selecting a coverage criterion.
- FR-021 Generation is deterministic for given model, criterion, seed.
- FR-022 Show coverage achieved per model, feature and component.
- FR-023 Regenerate and diff against existing test cases.
- FR-024 Export test cases as JSON, CSV, Gherkin.

### AI proposals
- FR-030 Propose models from a feature's scenario description or a textual requirement.
- FR-031 Propose missing states/transitions for an existing model.
- FR-032 Propose additional test cases (edge/negative) for a model.
- FR-033 Review UI: accept, edit, reject each proposal; accepted ones become entities.
- FR-034 Proposals show rationale and are labelled as AI-origin.

### Authentication and tenancy
See [08-usermanagement.md](08-usermanagement.md).
- FR-040 Validate a JWT on every request unless `--dev-mode` is set.
- FR-041 Read the token from the `Authorization: Bearer` header.
- FR-042 Verify the signature against a JWKS fetched from a configurable well-known URL.
- FR-043 Take the tenant from a configurable, possibly nested claim (default `tenant`); reject a token without one.
- FR-044 Scope every read and write to the caller's tenant.
- FR-045 Allow `GET` for the `User` role; require `Editor` for every mutating method.
- FR-046 Offer logout from the user profile in the top-right corner.
- FR-047 Perform the OIDC handshake in an APISIX gateway that forwards to the UI and the backend.
- FR-048 Never expose a token to the SPA.

## Non-functional
- NFR-001 Generate tests for a 200-state model in under 2 s.
- NFR-002 API p95 latency under 200 ms excluding AI calls.
- NFR-003 LLM provider configurable; no data sent without configuration.
- NFR-004 WCAG 2.1 AA for UI (editor offers keyboard alternative).
- NFR-005 All API input validated; secrets never logged.
- NFR-006 Optimistic concurrency via `version` on updates.
- NFR-007 Tenant separation is enforced server-side; a request can never read or
  write another tenant's data, whatever ids it supplies.
