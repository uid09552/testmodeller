# Proposal

## Why

TestModeller covers functional behaviour (state machines → test cases) but has nothing for security. Teams do threat modelling in workshops, spreadsheets or separate tools, and the resulting threats, risks and controls never become tests anyone runs. A threat model per project — architecture, trust zones, assets, threats, risks, controls — with security test cases derived from it closes that gap and builds on the tool's core strength: turning a model into tests.

This change delivers the MVP (roadmap phase P1 of the threat-modelling vision) plus security test generation. Later phases are listed under *Not in this change* so the vision stays visible without inflating this change.

## What Changes

- **Threat model in the project tree**: a shield-icon item under each project that has one (created from the project's right-click menu, no nav item), opening a project-scoped threat model workspace with tabs: Overview, Diagram, Components, Data Flows, Zones, Assets, Threats, Risks, Controls, Security Tests.
- **One threat model per project**, with scope, objectives, assumptions and exclusions.
- **Architecture registry**: register *architecture components* (new entity, not the existing `Component`) with a type from a fixed taxonomy (web frontend, API, service, database, identity provider, external entity, CI/CD, …), owner, technology, trust zone, exposure (internet-facing or not), criticality and an optional link to an existing Component/Feature.
- **Relationships and data flows** between architecture components: type (calls, reads/writes, authenticates through, depends on, …), protocol, data exchanged (linked assets), authentication, encryption in transit. Several flows may connect the same pair.
- **Exposure zones** with a trust level (0–4). A flow whose endpoints are in zones of different trust level **crosses a trust boundary**; crossings are computed, never entered by hand.
- **Asset inventory**: assets are independent entities (category, owner, C/I/A requirement each Low–High, personal data flag) linked to many components and flows.
- **Data-flow diagram canvas**: components as shapes, flows as arrows, zones as labelled regions, built from the model editor's canvas helpers (shapes, routing, snapping, viewport). Layout is presentation-only and stored opaque, like ADR 0010.
- **Threats**: structured records (STRIDE category, title, scenario, preconditions, target component/flow/asset, status: candidate → open → mitigated / accepted / dismissed). Created manually or as **candidates from STRIDE rules** applied to the architecture (e.g. "flow crosses a trust boundary without authentication"). Each candidate states the rule and the facts that triggered it; nothing becomes an open threat without the user confirming it.
- **Risk assessment** on each threat: likelihood × impact on a 5 × 5 matrix, inherent and current score, configurable thresholds per threat model (default Low 1–4, Moderate 5–9, High 10–16, Critical 17–25), treatment (mitigate / avoid / transfer / accept) with owner and acceptance rationale.
- **Security controls**: catalog per threat model (type preventive / detective / corrective / compensating, status planned / implemented / verified), linked to threats. Current risk is entered by the analyst; the tool never lowers a score just because a control is linked.
- **Security test cases** as a separate list in the threat model (not feature test cases): title, steps, expected result, linked threat and control, status (draft / ready), last result (passed / failed / not run, recorded manually). **Generate** derives draft security tests from open threats and their controls via STRIDE test templates; drafts are reviewed before becoming ready. Export to CSV and Gherkin.
- **Overview**: counts per component/threat status, risk distribution on the matrix, controls verified, threats without controls, controls without tests.
- **API**: new endpoints under `/projects/{projectId}/threat-model` and child resources (contract change — requires explicit approval before `openapi.yaml` / `05-api.md` are edited, AGENTS.md rule 6).
- **Docs**: new `docs/specification/09-threat-modeling.md`; glossary, domain model, requirements (FR-060…), UI spec updated; ADR for "architecture component is a separate entity" and "security tests are a separate list".

### Not in this change (later phases)
Attack-path analysis and blast radius, dependency risk propagation, what-if simulation, residual risk from control effectiveness, evidence management and approvals workflow, LINDDUN/PASTA/ATT&CK, AI-assisted threat and diagram extraction, imports (OpenAPI, SBOM, scanners, IaC), framework mappings (ASVS, NIST, ISO), portfolio dashboards, integrations (Jira, CI), PDF reports.

## Capabilities

### New Capabilities
- `threat-model-workspace`: the threat model item in the project tree and its create/delete menus, the per-project threat model (scope, assumptions), its tabs and the Overview summary.
- `threat-model-architecture`: architecture components, relationships/data flows, exposure zones, assets, and computed trust-boundary crossings.
- `threat-model-diagram`: the interactive data-flow diagram canvas and its persisted layout.
- `threat-identification`: threat records, their lifecycle, and STRIDE rule-based candidate generation with explanations.
- `threat-risk-assessment`: likelihood/impact scoring, the 5 × 5 matrix, thresholds, inherent and current risk, treatment.
- `security-controls`: the control catalog and its links to threats.
- `security-test-cases`: the separate security test list, generation from threats/controls, manual results, export.

### Modified Capabilities
None. The model editor, feature test cases, coverage and traceability are unchanged; the only shared surface is the navigation bar, which gains an item.

## Impact

- **Backend**: `domain` (new `threat` module: entities, enums, trust-boundary computation, STRIDE rules, risk scoring, test templates — pure and unit-tested), `storage` (migration `0008_threat_model.sql`, new `threat_model` module, tenant-scoped), `api` (new handlers and DTOs, routes, tenant guard), JSON export/import optionally extended later (not in this change).
- **Frontend**: new `features/threat-model/` (routes, store, pages per tab, diagram canvas reusing `node-shapes`, `edge-geometry`, `style-render`, `snapping`, `viewport`), sidenav item, API client.
- **Contract**: `openapi.yaml` and `05-api.md` gain the threat-model resources — **requires user approval** before editing.
- **Docs**: `00-overview.md` scope, `01-glossary.md` (architecture component, exposure zone, trust boundary, asset, threat, risk, control, security test case), `02-domain-model.md`, `03-requirements.md`, `06-ui.md`, new `09-threat-modeling.md`, two ADRs.
- No change to existing tables, endpoints or the model editor's behaviour.
