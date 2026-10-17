# Tasks

## 1. Contract and documentation (approval gate)

- [x] 1.1 Draft the endpoints and schema fields from design D8 as a patch to `docs/specification/openapi.yaml` and `05-api.md`, present it to the user, and apply it only after explicit approval (AGENTS.md rule 6); decide nested vs top-level proposal routes; verify `openapi.yaml` lints after applying
- [x] 1.2 Extend `07-ai-integration.md` (threat model kinds, context rules, opt-in, forbidden actions), `09-threat-modeling.md` (origin `ai`, `aiEnabled`), `06-ui.md` (chat panel, badge, enable dialog) and `03-requirements.md` (FR-08x mapped to the three specs); verify every spec requirement maps to an FR id
- [x] 1.3 Add ADR `0013-threat-model-proposals-store.md` (D2 separate store with shared jobs) and link it from `docs/adr/README.md`; verify links resolve

## 2. AI crate (`backend/crates/ai`)

- [x] 2.1 Add `tm` module: `TmProposalKind`, typed payloads with `deny_unknown_fields`, `TmContent`; verify serde tests incl. rejection of `status`, `evidence`, `treatment`, `acceptance`, `thresholds`, `delete` fields
- [x] 2.2 Add `TmContext` renderer with truncation (targets + one hop), description cap and `truncated` flag; verify unit tests for small, large and targeted models and that no tenant id, key or email can appear
- [x] 2.3 Add prompt templates `tm-system|architecture|threats|ratings|controls|security-tests|review.v1.md`, delimited-data rule and `PROMPT_VERSION` handling; verify prompt rendering snapshot tests
- [x] 2.4 Implement `parse_tm_proposals` validation (taxonomy, ranges, ≥ 1 target, ids present, unique names, in-payload name resolution) and `generate_tm_proposals` with one bounded retry; verify unit tests with a fake provider for each kind, retry success and retry failure
- [x] 2.5 Add prompt-injection fixtures (hostile descriptions, forbidden fields, foreign ids); verify none produces a persisted change
- [x] 2.6 Run `cargo fmt` and `cargo clippy -D warnings` on the crate; verify clean

## 3. Domain and storage

- [x] 3.1 Add `origin` value `ai` and `source` to threat, control, security test and architecture entities in `domain::threat`; verify serde and constructor tests
- [x] 3.2 Add the next free migration (`tm_proposals`, `threat_models.ai_enabled`, origin/source columns, CHECK updates); verify it applies on a fresh and on a populated testcontainers database
- [x] 3.3 Implement `tm_proposals` storage (create many, list with filters, get, status transitions, tenant scoping via threat model, cascade on threat model delete); verify integration tests
- [x] 3.4 Implement accept appliers per kind in one transaction (architecture name→id resolution in dependency order; threat; rating on one field; control create-or-link; security test as draft) re-validating against current state; verify integration tests incl. deleted-target refusal leaving the proposal pending and ratings leaving other fields unchanged
- [x] 3.5 Persist `aiEnabled` on the threat model with `If-Match`; verify integration test

## 4. API (`backend/crates/api`)

- [x] 4.1 Add DTOs and mappings for requests, proposals, `aiEnabled`, `origin`/`source`; verify mapping unit tests
- [x] 4.2 Implement request handler (checks provider ≠ none and `ai_enabled`, `previewContext`, job creation, rate limit, token budget) and run the job via the existing runner; verify API tests for refusal messages, preview without provider call and job result ids
- [x] 4.3 Implement list/get/accept/reject handlers under the tenant guard with edit-role checks; verify tenant-isolation and 404/422 tests
- [x] 4.4 Extend structured logging with kind, prompt version, model id, sizes, outcome and verify no content appears in logs
- [x] 4.5 Run `cargo fmt`, `cargo clippy -D warnings`, backend tests; verify clean

## 5. Frontend (`frontend/src/features/threat-model`)

- [x] 5.1 Extend API client and store (`aiEnabled`, proposals, jobs polling); verify store unit tests
- [x] 5.2 Build `ai-chat` panel (open/close from header, transcript, kind inference and selector, focus from selection, pending indicator, errors, disabled state); verify component tests for the `threat-model-ai-chat` scenarios
- [x] 5.3 Build cards per kind (architecture new-vs-existing list, threat, rating old-vs-new, control with link-existing, security test, review text with entity links) with Accept/Edit/Reject; verify component tests incl. accept-then-update ordering
- [x] 5.4 Add AI enable switch and notice (provider + endpoint host), context preview dialog and truncation message; verify component tests incl. role-disabled switch
- [x] 5.5 Add `AiBadge` to tables, detail view and report (if present); verify component tests
- [x] 5.6 Run `ng lint` and frontend tests; verify clean

## 6. Verification

- [x] 6.1 Walk through with a configured provider (or fake): enable AI, preview context, describe an architecture, accept some components, ask for threats and controls, rate a threat, run a review, reject a proposal; verify against every scenario in the three specs
- [x] 6.2 Verify disabled paths: provider `none`, AI off, forbidden-field answer, foreign-id answer, injection description
- [x] 6.3 Run `openspec validate add-threat-model-ai-agent`; verify valid
