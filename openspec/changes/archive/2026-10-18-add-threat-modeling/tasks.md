# Tasks

## 1. Contract and documentation (approval gate)

- [x] 1.1 Draft the threat-model endpoints and schemas from design D4 as a patch to `docs/specification/openapi.yaml` and `05-api.md`, present it to the user, and apply it only after explicit approval (AGENTS.md rule 6); verify `openapi.yaml` lints/parses (e.g. `npx @redocly/cli lint`) after applying
- [x] 1.2 Write `docs/specification/09-threat-modeling.md` (entities, crossings, STRIDE rules table, thresholds, test templates, staleness) and update `00-overview.md` scope, `01-glossary.md` (architecture component, exposure zone, trust boundary, asset, threat, risk, control, security test), `02-domain-model.md` (ER diagram + entities), `03-requirements.md` (FR-060…FR-07x mapped to the specs); verify every spec requirement maps to an FR id
- [x] 1.3 Add ADR `0011-threat-model-entities-and-security-tests.md` covering D1 and D2, linked from `docs/adr/README.md`; verify links resolve

## 2. Domain (`backend/crates/domain`, module `threat`)

- [x] 2.1 Add entity types and enums (component type taxonomy, zone kind, trust level 0–4, asset category, CIA levels, flow relationship/auth/encryption, threat category/status/origin, control type/category/status, security test type/status) with serde names matching the contract; verify with serde round-trip unit tests
- [x] 2.2 Implement `Architecture` aggregate, `crossing()` and `warnings()`; verify unit tests for every scenario in `threat-model-architecture` "Computed trust-boundary crossings" and "Missing-information warnings"
- [x] 2.3 Implement `risk.rs` (rating score, `Thresholds` validation and `level`, threat flags: no-control, planned-only, unsupported-reduction, overdue); verify unit tests for each `threat-risk-assessment` scenario plus a proptest that valid thresholds map every score 1–25 to exactly one level
- [x] 2.4 Implement `rules.rs` with `RULESET_VERSION`, the eight initial rules, facts, missing-information hints, control recommendations and rule keys; verify one unit test per rule scenario in `threat-identification` plus dedup and dismissed-key tests
- [x] 2.5 Implement `templates.rs` (templates per STRIDE category × target kind, one test per control where control-specific, template keys) and `staleness.rs`; verify unit tests for each `security-test-cases` generation and staleness scenario
- [x] 2.6 Run `cargo fmt` and `cargo clippy -D warnings` on the domain crate; verify clean

## 3. Storage (`backend/crates/storage`)

- [x] 3.1 Add migration `0008_threat_model.sql` per design D3 (tables, link tables, CHECKs, unique names, partial unique rule/template keys, `threat_model_id` indexes); verify migrations apply on a fresh testcontainers database
- [x] 3.2 Add `TenantScope::ThreatModel` and the tenant lookup; verify an integration test that another tenant's threat model resolves to not found
- [x] 3.3 Implement threat model CRUD (one per project, conflict on second create, cascade on project delete), scope fields, thresholds and layout with separate `layout_version`; verify integration tests incl. 412 on stale version
- [x] 3.4 Implement zones, components (links to same-project Component/Feature, `SET NULL` on delete), assets, flows (same-threat-model endpoints, cascade on component delete, delete-count query) with link-set replacement; verify integration tests for each architecture scenario that touches persistence
- [x] 3.5 Implement threats (display-id counter, targets, assets, controls links, rule keys, bulk status) and controls (counter, components, verified date); verify integration tests incl. id non-reuse and dismissed rule keys blocking re-suggestion
- [x] 3.6 Implement security tests (counter, steps, threat/control links, template keys, bulk status) and results (latest by date); verify integration tests incl. threat deletion leaving the test with no threat
- [x] 3.7 Implement `load_architecture()` / aggregate read used for computed fields and summary; verify an integration test that loads a 300-entity model in under 50 ms locally

## 4. API (`backend/crates/api`)

- [x] 4.1 Add DTOs (input and read, with computed fields) and explicit domain mappings; verify unit tests for mapping of crossings, warnings, flags and staleness
- [x] 4.2 Add handlers and routes for threat model, layout, summary and the seven collections, with tenant guard, Editor role on mutations, `If-Match` on PUT and RFC 7807 errors; verify API integration tests for create/read/update/delete, 403 for User role, 404 cross-tenant, 412 on stale version
- [x] 4.3 Add `POST …/threats/suggest` and `…/threats/bulk-status`; verify API tests that a second suggest creates nothing and dismissal needs a reason
- [x] 4.4 Add `POST …/security-tests/generate`, `…/bulk-status` and results endpoints; verify API tests for generation, skipped-duplicates count and candidates excluded
- [x] 4.5 Run `cargo fmt`, `cargo clippy --workspace -D warnings` and `cargo test --workspace`; verify all clean and green

## 5. Frontend foundation

- [x] 5.1 Add `features/threat-model/` with lazy routes `/threat-modeling/:projectId/:tab`, redirect to the first project, and register it in `app.routes.ts`; verify a route spec
- [x] 5.2 List the threat model under its project in the project tree (shield icon), with New / Open / Delete in the right-click menus and no sidenav item; verify the tree and sidenav specs
- [x] 5.3 Add `threat-model.api.ts` and `threat-model.store.ts` (parallel load, mutations from server responses, version handling, conflict message); verify store specs with an HTTP testing backend
- [x] 5.4 Add the workspace page with project dropdown, empty state with "Create threat model", keyboard tab list synced to the URL; verify page specs for each `threat-model-workspace` navigation scenario
- [x] 5.5 Add a reusable `entity-table` (sort, filter, keyboard row focus, Enter opens editor) and editor drawer; verify component specs for keyboard behaviour

## 6. Architecture tabs

- [x] 6.1 Zones tab (CRUD, trust level, delete clears zone of components); verify specs
- [x] 6.2 Components tab (type grouped select, zone, exposure, criticality, owner, link to Component/Feature picker, delete confirmation naming flow count); verify specs incl. duplicate-name error
- [x] 6.3 Assets tab (category, CIA, personal data, linked components and flows from both sides); verify specs for shared asset scenario
- [x] 6.4 Data Flows tab (source/target, relationship, protocol, auth, encryption, assets, crossing column and "crosses trust boundary" filter, warnings shown); verify specs
- [x] 6.5 Update `docs/specification/06-ui.md` with the Threat Modeling screen, tabs and architecture tables; verify the screen list mentions it

## 7. Threats, risks and controls tabs

- [x] 7.1 Threats tab: list with filters, candidate labelling, editor (category, scenario, targets, assets, controls, status, dismiss reason), bulk confirm/dismiss, "Suggest threats" with result message, explanation and recommendations panel with "Add as control"; verify specs for each `threat-identification` UI scenario
- [x] 7.2 Risks tab: inherent/current rating inputs, treatment with accept rationale and approver, due date and overdue marker, accessible 5 × 5 `risk-matrix` with inherent/current switch and cell filter, register and CSV export, thresholds editor with validation message; verify specs for each `threat-risk-assessment` scenario
- [x] 7.3 Controls tab: CRUD, status with verified date, linked threats and tests, filters (without threats, without tests, by status); threat flags "No control" / "Controls planned only" in the Threats tab; verify specs
- [x] 7.4 Overview tab: scope fields editor, counts, matrix, gap lists linking to the item, empty-model prompt; verify specs for each Overview scenario
- [x] 7.5 Extend `06-ui.md` with threats, risks, controls and overview behaviour; verify against specs

## 8. Security tests tab

- [x] 8.1 List with filters, editor (steps, threats, controls, type, status), stale badge with reason, latest result badge with symbol and word; verify specs
- [x] 8.2 "Generate tests" for selection or all, result summary (created/skipped), bulk "Mark ready"; verify specs that candidates are excluded and edits survive regeneration
- [x] 8.3 Record result dialog (passed/failed/not run, note, date) and "✗ Failing test" flag on threats; verify specs
- [x] 8.4 Client-side CSV and Gherkin export of the filtered list (`@sec-<id>`, threat tags, Given per precondition); verify export unit tests
- [x] 8.5 Extend `06-ui.md` with the security tests tab; verify against specs

## 9. Diagram tab

- [x] 9.1 `threat-diagram` component and `diagram.store.ts`: map components/flows to the model canvas helper types, render shapes by layer, labels, zone regions with trust level, flows with `edge-geometry`; verify render specs (database as cylinder, components inside their zone)
- [x] 9.2 Editing: type palette, drag-to-connect flow creation opening its editor, double-click/Enter opens editor, drag into/out of a zone region updates the component's zone, delete selection; verify specs for each editing scenario
- [x] 9.3 Boundary markers with accessible names, threats overlay toggle with count and highest level badge linking to the filtered Threats tab; verify specs
- [x] 9.4 Zoom/fit (Shift+1), pan, snapping, `canvas-minimap`, Tab/Shift+Tab selection; verify specs
- [x] 9.5 Layout `{v:1,…}` save via `PUT …/layout` (debounced), auto-placement of unknown entities without overlap, dropping deleted entries; verify specs for reload and API-created component scenarios
- [x] 9.6 Extend `06-ui.md` with the diagram; verify against specs

## 10. Integration checks

- [x] 10.1 Run `ng lint`, `ng test` and `ng build` for the frontend and `cargo fmt --check`, `cargo clippy --workspace -D warnings`, `cargo test --workspace` for the backend; verify all clean
- [x] 10.2 End-to-end walkthrough in the running stack (`make dev`): create a threat model, register 4 components in 2 zones with flows and an asset, suggest threats, confirm two, rate and treat them, add controls, generate and mark tests ready, record a failure, export Gherkin, reload the diagram; verify every step behaves as the specs say and that the Test Cases page, coverage and traceability are unchanged
- [x] 10.3 Run `openspec validate add-threat-modeling --strict`; verify it passes

## Workflow follow-up

- Archive the change after review with `/opsx:archive`, then verify the new capabilities appear under `openspec/specs/`.
- Follow-up changes for later phases: attack paths and blast radius, what-if simulation, evidence and approvals, AI-assisted threat suggestions (via proposals), imports (OpenAPI, SBOM, scanners), framework mappings, project export/import of threat models, shared diagram library extraction.
