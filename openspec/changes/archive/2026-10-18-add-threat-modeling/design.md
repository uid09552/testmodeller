# Design

## Context

- Backend: Rust workspace — `domain` (pure entities and rules), `generation`, `storage` (sqlx/PostgreSQL, embedded migrations up to `0007_model_layout.sql`), `api` (axum handlers, DTOs, tenant guard). Every entity is reached through a project; `TenantScope` resolves a path id to its tenant before a handler runs (ADR 0005). Updates need `If-Match` and return 412 on mismatch (NFR-006).
- Frontend: Angular standalone, signals, lazy feature routes under `ShellComponent`; the nav items are a static list in `core/layout/sidenav/sidenav.ts`.
- Model canvas: `features/models/components/canvas` is bound to `ModelEditorStore`, but its geometry helpers (`node-shapes`, `edge-geometry`, `snapping`, `viewport`, `style-render`) are pure functions typed against the store's `CanvasNode`/`CanvasEdge`/`Anchor` types, and `canvas-minimap` takes plain inputs.
- The glossary reserves "Component" for Project > Component; the new entity is the **architecture component** (user-facing label "Component" only inside the threat model workspace, where the context is unambiguous; API and code use `ArchComponent` / `arch-components`).
- The API contract may only be changed after explicit user approval (AGENTS.md rule 6). The endpoints below are the proposal for that approval.

## Goals / Non-Goals

**Goals:**
- All analysis (crossings, warnings, rule matches, risk levels, test templates, staleness) is pure, deterministic domain code with unit tests; storage and API only persist and expose it.
- The threat model is one aggregate per project that the workspace can load in a few requests.
- Reuse the model canvas's geometry rather than writing a second diagram engine.

**Non-Goals:**
- No change to existing tables, endpoints, the model editor or feature test cases.
- No inclusion of threat models in the project JSON export/import in this change.
- No AI involvement; the existing proposal mechanism is untouched.
- No extraction of a shared diagram library; that is a follow-up once both canvases exist.

## Decisions

### D1. Separate entities, not extended `Component`
`ArchComponent` is a new table with an optional `linked_component_id` / `linked_feature_id` (same project, `ON DELETE SET NULL`). Extending `Component` would force databases, identity providers and external entities into the test hierarchy. Recorded as an ADR.

### D2. Security tests are their own table
`security_tests` hang off the threat model, not off a feature. This keeps generation, coverage, traceability and result import of feature test cases unaffected, at the price of not reusing those pipelines (results are recorded manually; export is done client-side). Recorded as an ADR together with D1. Moving security tests into features later remains possible because each test carries its template, threat and control ids.

### D3. Schema (migration `0008_threat_model.sql`)
- `threat_models` (`project_id` UNIQUE, FK cascade; `tenant_id` copied like `test_results`; name, scope, objectives, assumptions, exclusions; `thresholds jsonb`; `layout jsonb`; `layout_version`; counters `next_threat_no`, `next_control_no`, `next_test_no`; `version`, timestamps).
- `tm_zones`, `tm_components`, `tm_assets`, `tm_flows`, `tm_threats`, `tm_controls`, `security_tests`, `security_test_results`: each with `threat_model_id` FK cascade, `version`, timestamps; names unique per threat model where the spec says so.
- Link tables with composite PKs and cascade on both sides: `tm_component_assets`, `tm_flow_assets`, `tm_threat_targets` (component or flow, CHECK exactly one), `tm_threat_assets`, `tm_threat_controls`, `tm_control_components`, `security_test_threats`, `security_test_controls`.
- `tm_rule_keys (threat_model_id, rule_key, threat_id NULL REFERENCES tm_threats ON DELETE SET NULL)`, PK `(threat_model_id, rule_key)`, implements "no duplicate candidates": a key is written with every rule-created threat and outlives the threat, so dismissed and deleted threats are never re-suggested. `security_tests.template_key` likewise (`template|threat|control`) with `WHERE status <> 'obsolete'`.
- Enum columns are `text` with CHECK constraints, matching existing migrations.
- Display ids (`THR-n`) come from the counters, incremented in the insert transaction, so deleted numbers are never reused.

### D4. API shape (pending approval)
Only the threat model is a new tenant scope; children are addressed under it, and storage checks that the child belongs to that threat model (404 otherwise). One new `TenantScope::ThreatModel`.

```
GET  POST            /projects/{projectId}/threat-model
GET  PUT  DELETE     /threat-models/{tmId}
PUT                  /threat-models/{tmId}/layout          (If-Match on layout_version)
GET                  /threat-models/{tmId}/summary
GET  POST            /threat-models/{tmId}/{zones|components|assets|flows|threats|controls|security-tests}
GET  PUT  DELETE     /threat-models/{tmId}/{collection}/{id}
POST                 /threat-models/{tmId}/threats/suggest
POST                 /threat-models/{tmId}/threats/bulk-status        {ids, status, reason?}
POST                 /threat-models/{tmId}/security-tests/generate    {threatIds?}
POST                 /threat-models/{tmId}/security-tests/bulk-status {ids, status}
POST GET             /threat-models/{tmId}/security-tests/{id}/results
```
Link sets (assets of a component, targets/controls of a threat, …) are arrays of ids in the parent's input DTO and replaced as a whole on PUT, so one editor save is one PUT. Read DTOs carry the computed fields: flows `crossing {crosses, direction, fromTrust, toTrust}`, entities `warnings[]`, threats `level`, `flags[]` (no-control, planned-only, unsupported-reduction, overdue, failing-test), tests `stale {reason}?` and `lastResult`.

Alternative considered: flat top-level `/arch-components/{id}` etc. with one tenant scope each — rejected as nine new scopes for no benefit.

### D5. Domain module `tm_domain::threat`
- `arch.rs`: entity types and `Architecture` (the aggregate read from storage); `crossing(flow, &Architecture)` and `warnings(&Architecture)`.
- `rules.rs`: `RULESET_VERSION` and `fn suggest(&Architecture, &[ExistingRuleKey]) -> Vec<Candidate>`; each rule is a struct with id, STRIDE category, a matcher returning facts and missing-information hints, and control recommendations. Rule key = `rule_id:target_kind:target_id`. Rules from the spec are the initial set.
- `risk.rs`: `Rating`, `Thresholds` (validation: contiguous 1–25, no overlap), `level(score)`, threat flags.
- `templates.rs`: test templates keyed by (STRIDE category, target kind), rendering steps from target, asset and control names; `fn generate(threat, controls, existing_keys) -> Vec<NewSecurityTest>`.
- `staleness.rs`: stale reason for a generated test given its threat links and control links.
Pure, `thiserror` errors, unit tests per rule and template; a proptest for thresholds.

Candidate texts are deterministic templates, not AI output, so AGENTS.md rule 8 is respected by status alone: candidates are persisted but never count as open threats until confirmed.

### D6. Computed on read, not stored
Crossings, warnings, levels, flags and staleness are recomputed from the aggregate on each read. Threat models are small (hundreds of entities), so loading the aggregate per request is well within NFR-002; this avoids invalidation bugs when zones or links change. If needed later, cache per `version`.

### D7. Frontend structure
`features/threat-model/`:
- `threat-model.routes.ts`: `/threat-modeling` → redirect to first project; `/threat-modeling/:projectId/:tab`.
- `services/threat-model.api.ts`; `state/threat-model.store.ts` (signals; loads threat model, all collections and summary in parallel; mutations update the store from the server response).
- `pages/workspace-page` with tab list; one component per tab under `components/` (overview, components-table, flows-table, zones-table, assets-table, threats, risks with `risk-matrix`, controls, security-tests), shared `entity-table` for sort/filter/keyboard and an editor drawer.
- `components/threat-diagram` with its own `diagram.store.ts`. It maps components/flows to the `CanvasNode`/`CanvasEdge` shapes and imports `node-shapes`, `edge-geometry`, `snapping`, `viewport` and `canvas-minimap` from `features/models` unchanged. Zones are rectangles in the layout; a component's zone is decided by the region containing its centre on drop (smallest region wins), then saved through the normal component PUT. Layout format `{v:1, nodes, zones, edges}` saved via `PUT …/layout`, debounced like the model editor's autosave.
- Client-side exports (risk register CSV, security tests CSV/Gherkin) in `state/exports.ts`, mirroring the Test Cases page.
- Sidenav: one new item "Threat Modeling" (shield icon) after Traceability.

Alternative considered: generalise the model canvas into a shared diagram component first — rejected for this change (large refactor of a stable editor, AGENTS.md rule 4).

### D8. Thresholds and levels
Stored on the threat model as `[{level, min, max}]`; validated in domain on PUT; levels computed on read so a threshold change reclassifies everything immediately.

## Risks / Trade-offs

- [Contract approval may reshape endpoints] → Tasks start with the contract proposal; backend/frontend tasks follow approval.
- [Workspace scope is large for one change] → Tasks are ordered so each tab ships behind the nav item incrementally; the Diagram tab is last and the tables alone are usable.
- [Importing model-canvas helpers couples the two features] → Only pure helpers are imported; a follow-up extracts them to `shared/diagram`.
- [Rule candidates may be noisy] → Dedup by rule key, dismissals are remembered, bulk confirm/dismiss, and every candidate explains its trigger and what would change it.
- [Manual current-risk ratings can be optimistic] → The unsupported-reduction warning; no automatic reduction.
- [Security tests do not feed coverage/results import] → Accepted by the user's choice of a separate list; ADR records how to merge later.
- [Aggregate load per request] → Bounded by model size; add an index on every `threat_model_id`.

## Migration Plan

Additive only: migration `0008` creates new tables; no existing data changes. Rollback is dropping the new tables (no other table references them). The nav item ships with the first usable tab.

## Open Questions

- Shield icon path and exact palette type icons — decided during UI implementation.
- Whether `ON DELETE` of a linked `Component`/`Feature` should warn in the Explorer — currently silent `SET NULL`; can be refined without changing these specs.
