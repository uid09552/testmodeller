# Tasks

## 1. Contract and documentation (approval gate)

- [x] 1.1 Draft the API additions from design D8 as a patch to `docs/specification/openapi.yaml` and `05-api.md`, present it to the user, and apply it only after explicit approval (AGENTS.md rule 6); verify `openapi.yaml` lints after applying
- [x] 1.2 Extend `docs/specification/09-threat-modeling.md` (residual rating and flags, evidence, acceptance expiry and effective status, baselines, completeness findings, detail fields, grandfathering) and update `02-domain-model.md`, `03-requirements.md` (new FR ids mapped to the six specs), `06-ui.md`; verify every spec requirement maps to an FR id
- [x] 1.3 Add ADR `0012-residual-risk-and-baselines.md` (D1 analyst-entered residual; D4 immutable JSON baselines) and link it from `docs/adr/README.md`; verify links resolve

## 2. Domain (`backend/crates/domain`, module `threat`)

- [x] 2.1 Add residual rating, confidence, classification, evidence-kind and acceptance types with serde names matching the contract; verify serde round-trip tests
- [x] 2.2 Extend `risk.rs`: residual score/level, flags `residual-above-current`, `unsupported-residual`, `acceptance-expired`, `verification-overdue`, `verified-without-evidence`, effective status with injected clock; verify unit tests for every `residual-risk`, `control-evidence` and `risk-acceptance-governance` scenario
- [x] 2.3 Implement acceptance validation (rationale, approver, expiry in (today, today+3y]) and `can_verify(evidence_count)`; verify unit tests for refusal scenarios
- [x] 2.4 Implement affected-boundaries and the sensitive-data-crossing flow warning in `arch.rs`; verify unit tests for the `threat-model-detail-fields` scenarios incl. direction
- [x] 2.5 Implement `completeness()` and `diff(snapshot, live)` (added/removed/changed, level histograms); verify unit tests per `threat-model-baselines` comparison scenario and `tra-report` completeness scenarios
- [x] 2.6 Run `cargo fmt` and `cargo clippy -D warnings` on the domain crate; verify clean

## 3. Storage (`backend/crates/storage`)

- [x] 3.1 Add migration `0009_tra_assurance.sql` per design (threat/asset/flow/control columns, `tm_evidence`, `tm_baselines`, indexes on `threat_model_id`); verify it applies on a fresh and on an 0008-populated testcontainers database
- [x] 3.2 Decide single vs two evidence tables (design D2) and implement evidence CRUD with cascade on control/result delete and verified→implemented downgrade on last-evidence delete; verify integration tests incl. no orphans
- [x] 3.3 Persist new threat, asset, flow and control fields and the acceptance record; verify integration tests incl. 412 on stale version
- [x] 3.4 Implement baselines (create from aggregate, unique name, cap 50, list, get, delete, cascade with threat model); verify integration tests incl. immutability and that later edits do not alter the snapshot
- [x] 3.5 Extend summary with completeness findings and expiring acceptances; verify integration test on a seeded model

## 4. API (`backend/crates/api`)

- [x] 4.1 Add DTO fields, evidence and baseline DTOs with explicit domain mappings; verify mapping unit tests for effective status and new flags
- [x] 4.2 Implement evidence, baseline and compare handlers under the `ThreatModel` tenant scope with role checks as for other threat-model writes; verify tenant-isolation and 404/422/409 tests
- [x] 4.3 Reject `verified` without evidence (422) and accept-without-expiry (422) with clear messages; verify API tests
- [x] 4.4 Run `cargo fmt`, `cargo clippy -D warnings`, backend test suite; verify clean

## 5. Frontend (`frontend/src/features/threat-model`)

- [x] 5.1 Extend the API client and store with new fields, evidence and baselines; verify store unit tests
- [x] 5.2 Risks tab: residual rating inputs, matrix view selector (inherent/current/residual), register columns, new flags; verify component tests for `residual-risk` scenarios
- [x] 5.3 Acceptance dialog with approver, expiry and renewal, effective-status display, Overview expiring list; verify component tests
- [x] 5.4 Controls tab: evidence list/add/delete, verify action gated on evidence, review-by date, overdue flag; verify component tests
- [x] 5.5 Threat, asset and flow editors: new detail fields, affected boundaries, entry-point marker on table and diagram; verify component tests
- [x] 5.6 Baselines page: create, list, open read-only (banner, disabled edits), compare view, delete; verify component tests
- [x] 5.7 Report page with print stylesheet and Markdown export for live model and baseline; verify unit tests of the exporter and section order
- [x] 5.8 Overview completeness findings with links; verify component tests
- [x] 5.9 Run `ng lint` and frontend tests; verify clean

## 6. Verification

- [x] 6.1 End-to-end walk-through: rate residual, add evidence and verify a control, accept with expiry, create a baseline, change a threat, compare, print report; verify against each scenario in the six specs
- [x] 6.2 Update `docs/specification` status and run `openspec validate add-tra-assurance`; verify valid
