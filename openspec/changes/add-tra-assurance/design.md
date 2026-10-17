# Design

## Context

`add-threat-modeling` (complete, not yet archived) provides the `domain::threat` module, migration `0008_threat_model.sql`, `tm_*` tables, the API under `/threat-models/{tmId}` and the Angular `features/threat-model`. Computed fields (crossings, warnings, levels, flags, staleness) are derived on read from the aggregate (its D6). Ratings are analyst-entered; linking controls never changes a score. The API contract may only change after explicit user approval (AGENTS.md rule 6); this document proposes the changes for that approval.

## Goals / Non-Goals

**Goals:** make the output defensible — residual risk, evidence-backed verification, time-limited acceptance, point-in-time baselines, a report — with pure, unit-tested domain rules and additive, optional API fields.

**Non-Goals:** derived residual risk, attack paths, authenticated approval workflow, file storage, server-side PDF, framework mappings, integrations.

## Decisions

### D1. Residual rating is a third stored rating
Columns `residual_likelihood`, `residual_impact` on `tm_threats` with the same pair-CHECK as the other ratings. Rejected: deriving it from control status/effectiveness — the base design already rejects fixed-percentage reduction, and an effectiveness model needs its own design. Flags stay advisory: `residual-above-current` and `unsupported-residual` (residual < current and no verified linked control). ADR: "residual risk is analyst-entered".

### D2. Evidence is references, not files
New table `tm_evidence (id, threat_model_id, owner_kind CHECK IN ('control','test-result'), owner_id, kind, title, link, note, evidence_date, reviewer, version, timestamps)` with CHECK `link IS NOT NULL OR note IS NOT NULL`. `owner_id` is not a FK (two possible parents); cascades are implemented in storage deletes of control/result inside their transaction, plus an orphan check in tests. Alternative: two tables (`tm_control_evidence`, `tm_result_evidence`) with real FKs — preferred if sqlx ergonomics are equal; decide in task 3.2. No binary storage (avoids upload, virus scanning, retention).

`tm_controls` gains `review_by date`. The "verified requires evidence" rule lives in domain (`controls::can_verify(evidence_count)`), enforced in the PUT handler and on evidence delete (auto-downgrade to `implemented` in the same transaction).

### D3. Acceptance governance on the threat row
`tm_threats` gains `accepted_on date`, `acceptance_expires date`. Domain rule: accept ⇒ rationale + approver + expiry in (today, today+3y]. Expiry is evaluated on read: stored `status` stays `accepted`, the read DTO reports `effectiveStatus: open` plus flag `acceptance-expired`. No background job, so nothing mutates silently and nothing needs scheduling; the cost is that status filters must use effective status (one helper, tested). "Today" is injected (`Clock`) like `overdue` in the base design.

### D4. Baselines are immutable JSON snapshots
`tm_baselines (id, threat_model_id, name, note, created_by, created_at, schema_version, snapshot jsonb)` with `UNIQUE (threat_model_id, name)`. The snapshot is the serialised read aggregate (the same DTO the live model returns, minus computed fields, which are recomputed when viewing so the baseline report uses current rule versions — recorded in the snapshot as `rulesetVersion`). Rejected: temporal tables / per-row history — large, and the need is "what did we sign off", not full audit. Size: hundreds of entities ⇒ tens to hundreds of KB per baseline; cap at 50 baselines per threat model.

Viewing: `GET …/baselines/{id}` returns the aggregate in the live shape, so the frontend renders it through the same store with an `readOnly` flag. Comparison: `GET …/baselines/{id}/compare` runs `domain::threat::diff(snapshot, live)` — pure, by entity id; lists added/removed/changed threats (status, three levels, treatment), control status changes, level histograms.

### D5. Report is a frontend view over the aggregate
No server rendering. `features/threat-model/pages/report-page` renders the aggregate (live or baseline) with a print stylesheet (`@media print`) and a Markdown exporter in `state/exports.ts`. Deterministic text only. Rejected: server-side PDF (new dependency and fonts for little value over browser print).

### D6. Completeness findings are computed in domain
`domain::threat::completeness(&Architecture, &[Threat], …, today) -> Vec<Finding {code, entityRef}>` and included in the summary response. Not a score — a list — so there is nothing to game.

### D7. Detail fields
Additive nullable columns: `tm_threats(threat_source, detection, existing_mitigations, confidence)`, `tm_assets(classification, data_location, retention, encryption_at_rest default 'unknown')`, `tm_flows(data_classification, authorization_model, entry_point boolean default false)`. Affected trust boundaries of a threat and the "sensitive data towards less trusted zone" warning are computed on read from existing `crossing()`; asset sensitivity = classification confidential/restricted or `personal_data`. No change to STRIDE rules in this change.

### D8. API shape (pending approval)
Additions only; all new request fields optional.
```
PUT   /threat-models/{tmId}/threats/{id}       + residual{likelihood,impact}, acceptance{approver,expires}, threatSource, detection, existingMitigations, confidence
PUT   /threat-models/{tmId}/controls/{id}      + reviewBy; status=verified rejected with 422 when no evidence
GET POST        /threat-models/{tmId}/controls/{id}/evidence
GET POST        /threat-models/{tmId}/security-tests/{id}/results/{rid}/evidence
DELETE          /threat-models/{tmId}/evidence/{id}
GET POST        /threat-models/{tmId}/baselines
GET DELETE      /threat-models/{tmId}/baselines/{id}
GET             /threat-models/{tmId}/baselines/{id}/compare
GET             /threat-models/{tmId}/summary   + completeness[], expiring acceptances
```
Read DTOs add: threat `residual`, `residualScore/Level`, `effectiveStatus`, `affectedBoundaries[]`, flags; control `evidenceCount`, flag `verification-overdue`; flow warning list. Tenant scope: evidence and baselines are children of `ThreatModel` scope, storage checks parentage (404 otherwise).

## Risks / Trade-offs

- [Analyst-entered residual can be optimistic] → advisory flags and report lists them; derivation deferred.
- [Evidence is only a reference] → documented limitation; audit value is traceability, not custody.
- [Effective-status-on-read can be missed by a new query] → single domain helper; storage filters by stored status only for ids, DTO layer applies effective status; tests for list filter.
- [Snapshot JSON ages with schema] → `schema_version` in snapshot; reader tolerates missing fields; migration tests with a v1 fixture.
- [Contract approval may reshape endpoints] → tasks begin with the approval gate.
- [Depends on `add-threat-modeling` being archived/merged] → no code dependency issue (already in repo); spec deltas fold in after archive.

## Migration Plan

`0009_tra_assurance.sql` is additive: new nullable columns/defaults, `tm_evidence`, `tm_baselines`. Existing verified controls have no evidence: they are **grandfathered** — status is kept, and they show a `verified-without-evidence` flag and a completeness finding instead of being downgraded. Rollback drops the new objects.

## Open Questions

- Is `accepted_by` free text enough, or should it map to a user once authentication exists? Left as text.
- Maximum acceptance horizon (3 years) and baseline cap (50) are defaults; configurable per threat model later if needed.
