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

### Traceability
- FR-025 A test case has an optional implementation link and an optional backlog item link, each a full http(s) URL stored as its own field.
- FR-026 A backlog item (the requirement, identified by its URL) is traced to its test cases, their assigned states and transitions, models, features and components, and implementation links; URLs differing only by trailing slash, fragment or host case are one item.
- FR-027 Identify traceability gaps: test cases without a backlog link, test cases without an implementation link, and backlog items whose test cases cover no model element.
- FR-028 Show the trace as a filterable, searchable matrix and export it as CSV; only data of the caller's tenant and project is included.

### Test results
- FR-050 Import JUnit XML and Cucumber JSON result files for a project; reject malformed, unknown-format or oversized files (10 MB) without recording anything.
- FR-051 Match each result to a test case of the project by its `@tm-<id>` tag, otherwise by exact unique name; report recorded, duplicate, unmatched and ambiguous results. Gherkin and CSV exports carry the tag.
- FR-052 Keep every imported result with its run; the latest by execution time is the test case's result. Re-importing the same file records nothing new.
- FR-053 Show the latest result on test cases, in lists and on state chips with a text label, and count passing covered elements in coverage. Results never cross tenants or projects.

### AI proposals
- FR-030 Propose models from a feature's scenario description or a textual requirement.
- FR-031 Propose missing states/transitions for an existing model.
- FR-032 Propose additional test cases (edge/negative) for a model.
- FR-033 Review UI: accept, edit, reject each proposal; accepted ones become entities.
- FR-034 Proposals show rationale and are labelled as AI-origin.

### Threat modeling
See [09-threat-modeling.md](09-threat-modeling.md).
- FR-060 Offer a "Threat Modeling" navigation item opening a per-project workspace; a project has at most one threat model, created explicitly, with scope, objectives, assumptions and exclusions.
- FR-061 Register architecture components with a type from a fixed taxonomy, owner, technology, criticality, internet exposure, zone and an optional link to a Component or Feature of the same project.
- FR-062 Define exposure zones with a kind and a trust level 0–4.
- FR-063 Register assets (category, owner, C/I/A requirement, personal data) as independent entities linked to any number of components and flows.
- FR-064 Register data flows between components (relationship, protocol, authentication, encryption, assets); several flows may join the same pair.
- FR-065 Compute trust-boundary crossings of flows from zone trust levels on every read.
- FR-066 Warn about missing information (no zone, no owner, internet-facing without flow, undocumented authentication or encryption on a crossing, unlinked asset).
- FR-067 Edit the architecture on a data-flow diagram with zone regions, crossing markers and a threat overlay; persist its layout as presentation only.
- FR-068 Record threats (STRIDE category, scenario, targets, assets) with display ids and the lifecycle candidate → open → mitigated/accepted/dismissed.
- FR-069 Suggest candidate threats from versioned STRIDE rules, explain each match, never re-suggest a rule match on the same target, and require a user to confirm a candidate.
- FR-070 Rate inherent and current risk as likelihood × impact on a 5 × 5 matrix with configurable thresholds; never lower a rating automatically.
- FR-071 Record risk treatment, owner, due date and acceptance with rationale and approver; show the matrix and register and export the register as CSV.
- FR-072 Maintain a security control catalog linked to threats and components, create controls from rule recommendations, and flag threats without (implemented) controls.
- FR-073 Keep security tests as a separate list per threat model with steps, threats, controls, type and status.
- FR-074 Generate draft security tests from threats and controls with versioned templates, without duplicates; mark generated tests stale when their threat or control link goes away.
- FR-075 Record manual results of security tests and export the filtered list as CSV and Gherkin.
- FR-076 Summarise the threat model: counts, crossings, threats by status and level, matrix, verified controls, threats without controls, controls without tests.
- FR-077 Rate a residual risk per threat (likelihood × impact, analyst-entered, never derived); flag a residual above the current rating and a residual below it without a verified control; offer inherent, current and residual views of the matrix and register. (`residual-risk`)
- FR-078 Attach evidence (kind, title, link or note, date, reviewer) to controls and security test results; a control becomes verified only with evidence, and loses it again when its last evidence is deleted. (`control-evidence`)
- FR-079 Record a review-by date for verified controls and flag overdue verification. (`control-evidence`)
- FR-080 Record risk acceptance with rationale, approver, date and a mandatory expiry of at most 3 years; an expired acceptance reads as open with a flag; list acceptances expiring within 30 days. (`risk-acceptance-governance`)
- FR-081 Freeze the threat model as a named immutable baseline, open it read-only, compare it with the live model and delete it. (`threat-model-baselines`)
- FR-082 Produce a printable assessment report (live or baseline) with a sign-off block and export it as Markdown. (`tra-report`)
- FR-083 Show completeness findings on the Overview as a list, not a score. (`tra-report`)
- FR-084 Record threat source, detection, existing mitigations and confidence on threats, classification, location, retention and encryption at rest on assets, data classification, authorization model and entry-point on flows; warn when sensitive data flows towards a less trusted zone. (`threat-model-detail-fields`)
- FR-085 Request AI proposals for a threat model (architecture, threats, ratings, controls, security tests, review) as an asynchronous job; refused without a provider or without AI enabled for the threat model. (`threat-model-ai-proposals`)
- FR-086 Validate AI output against the domain rules with one bounded retry, accept or reject proposals individually (editing allowed), revalidating on accept; accepting a rating changes only that rating. (`threat-model-ai-proposals`)
- FR-087 Mark entities created from AI proposals with origin `ai` and the model id, and keep pending proposals out of counts and matrices. (`threat-model-ai-proposals`)
- FR-088 Offer a chat panel in the threat model workspace with kind inference, focus on a selected entity, proposal cards and clear error and disabled states. (`threat-model-ai-chat`)
- FR-089 Keep AI assistance off per threat model until enabled, show and cap the context sent, treat model text as data, forbid AI from verifying controls, adding evidence, accepting or dismissing risks, setting treatments or thresholds and deleting anything, and respect budgets and rate limits. (`threat-model-ai-safeguards`)

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
- NFR-004 WCAG 2.1 AA for UI (editor offers keyboard alternative: the table view and the canvas keyboard shortcuts, see 06-ui.md).
- NFR-005 All API input validated; secrets never logged.
- NFR-006 Optimistic concurrency via `version` on updates.
- NFR-007 Tenant separation is enforced server-side; a request can never read or
  write another tenant's data, whatever ids it supplies.
