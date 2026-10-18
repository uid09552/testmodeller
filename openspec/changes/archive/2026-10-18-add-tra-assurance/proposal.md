# Proposal

## Why

The threat model delivered by `add-threat-modeling` can identify, rate and treat threats, but its output is not yet defensible as a threat and risk analysis (TRA). It stops at *inherent* and *current* risk: there is no residual risk, "verified" controls carry no evidence, a risk acceptance is a free-text name that never expires, nothing records what was assessed at a given release, and there is no report to hand to a reviewer or auditor. Threat records also lack fields a reviewer expects (threat source, affected trust boundary, detection and response, confidence). Closing these gaps is the smallest step that turns the register into an assessment someone can sign off.

## What Changes

- **Residual risk**: each threat gets a third, analyst-entered rating (likelihood × impact) meaning "expected risk once planned measures are implemented and verified". Same thresholds and matrix as the other ratings. Nothing is derived automatically; flags warn about unsupported claims (see design).
- **Evidence**: controls and security-test results can carry evidence records (kind, title, link or note, date, reviewer). A control can only be set to `verified` with at least one evidence record. Verification has a date and an optional review-by date; overdue verification is flagged.
- **Risk acceptance governance**: acceptance records approver, date and a mandatory expiry. An expired acceptance puts the threat back to open with a "re-review" flag. Optional per-threat reassessment note.
- **Baselines**: a user can freeze the whole threat model as a named, immutable baseline (e.g. "Release 2.3 review"), list baselines, open one read-only, and see a comparison against the live model (threats added/removed/changed level, controls newly verified, risk counts).
- **TRA report**: a printable report view (scope, assumptions, architecture summary, risk register with inherent/current/residual, treatments, controls with evidence, open flags, sign-off block) exportable as Markdown and print-to-PDF. Works on the live model or a baseline.
- **Threat and asset detail fields**: threat source, detection opportunities, existing mitigations, confidence (low/medium/high); asset classification, data location, retention, encryption at rest. Flow gets data classification, authorization model and an entry-point flag.
- **Completeness**: Overview shows a completeness indicator (components without owner/zone, threats without rating, controls without evidence, acceptances expiring).
- **API**: new fields and endpoints under `/threat-models/{tmId}` (contract change — requires explicit approval before `openapi.yaml` / `05-api.md` are edited, AGENTS.md rule 6).
- **Docs**: extend `docs/specification/09-threat-modeling.md`, requirements (FR-07x), UI spec; ADR for "residual risk is analyst-entered, not derived" and "baselines are immutable JSON snapshots".

### Assumptions (to confirm at review)
- Output is for the team and for reviewers/auditors, but there is no authenticated approval workflow: approver is a recorded name, as in `add-threat-modeling`.
- Residual risk is analyst-entered; derivation from control effectiveness is a later change.
- PDF is produced by the browser's print dialog, not server-side.

### Not in this change
Attack paths and blast radius, what-if simulation, control-effectiveness model, threat pattern libraries beyond the 8 rules, LINDDUN/PASTA/ATT&CK, framework mappings (ASVS, NIST, ISO), imports (OpenAPI, SBOM, IaC, scanners), integrations (Jira, CI), comments/notifications, authenticated sign-off workflow, AI assistance.

## Capabilities

### New Capabilities
- `residual-risk`: the residual rating on a threat, its flags and its place in the matrix and register.
- `control-evidence`: evidence records on controls and test results, verification rules and review dates.
- `risk-acceptance-governance`: approver, date, expiry and re-review of accepted risks.
- `threat-model-baselines`: immutable named snapshots, read-only viewing and comparison with the live model.
- `tra-report`: the printable/exportable assessment report and the completeness indicator.
- `threat-model-detail-fields`: additional threat, asset and flow attributes.

### Modified Capabilities
None in `openspec/specs/`. The base capabilities (`threat-risk-assessment`, `security-controls`, …) still live in the unarchived `add-threat-modeling` change; archive it first so these specs can later be folded into them.

## Impact

- **Backend**: `domain::threat` (residual rating, flags, expiry rules, baseline diff — pure), `storage` (migration `0009_tra_assurance.sql`: columns on threats/assets/flows/controls, `tm_evidence`, `tm_baselines`), `api` (DTO fields, evidence and baseline endpoints).
- **Frontend**: Risks tab (third rating, matrix toggle), Controls tab (evidence), Threat/asset/flow editors, new Baselines and Report views, Overview completeness.
- **Contract**: `openapi.yaml`, `05-api.md` — requires user approval.
- **Docs**: spec 09, requirements, UI spec, two ADRs.
- No breaking change to existing endpoints; all new fields are optional.
