# Design

## Context

The model editor's assistant (ADR 0003, 0006, 0007; `07-ai-integration.md`) works as: `POST /ai/proposals` → async job (`jobs` table, polled via `GET /jobs/{id}`) → rows in `proposals` (`project_id`, `feature_id`, `kind`, `payload`, `rationale`, `source`, `status`) → accept/reject. The `ai` crate holds the `LlmProvider` trait with Anthropic and OpenAI-compatible adapters, versioned prompts (`prompts/*.v1.md`), `generate_proposals` (render → complete → parse/validate → one retry with the validation error) and `ProposalContent` types. Provider configuration is environment-driven (`TM_AI_*`). The editor needs a context-sync service (ADR 0006) because its model lives in the browser; the threat model lives in the backend and needs none.

Constraints: AGENTS.md rule 8 (no accepted data without approval), rule 6 (contract change needs approval), `07-ai-integration.md` safety rules (no auto-accept, untrusted content, no tool calls, budgets).

## Goals / Non-Goals

**Goals:** the same assistant experience in the threat model workspace; reuse provider, retry, prompt versioning and job mechanics; strong grounding and validation; privacy opt-in appropriate for security data.

**Non-Goals:** conversation history, tool use or autonomous loops, image/document ingestion, replacing the deterministic STRIDE rules, AI-computed risk or status.

## Decisions

### D1. Extend the `ai` crate, do not fork it
Add `ai::tm` with `TmContext` (renderer), `TmProposalKind`, `TmContent` and `generate_tm_proposals(provider, kind, request, max_tokens)` mirroring `generate_proposals`. Prompt templates `tm-architecture|threats|ratings|controls|security-tests|review.v1.md` plus a shared `tm-system.v1.md` (security-analyst role, JSON-only output, data-not-instructions rule). `PROMPT_VERSION` is recorded per kind. The provider trait and adapters are untouched.

### D2. Separate proposal store: `tm_proposals`
The existing `proposals` table requires `project_id` and ties to features/models, and its payload kinds are test-modelling specific. `tm_proposals (id, threat_model_id FK cascade, kind, status pending|accepted|rejected, payload jsonb, rationale, source, target_ids uuid[], resulting_entity_ids uuid[], reject_reason, prompt_version, created_at)`. The job mechanism is reused as is (`jobs.proposal_ids` holds tm proposal ids; the job result is kind-agnostic). Recorded in an ADR. Rejected: widening `proposals` with nullable threat-model columns — it would loosen existing constraints for both features.

### D3. Payloads reference by id and by in-payload name
`tm-architecture` payload `{zones[], components[], assets[], flows[]}` where new things refer to each other by `name` and to existing entities by `existingId`. On accept the server resolves names → new ids in dependency order (zones → assets → components → flows) inside one transaction, applying the normal create validation (unique names, taxonomy, same-model endpoints). Name clash with existing entity ⇒ the validator marks it `existing` and the card offers "link to existing" or "rename". Other kinds contain only existing ids; validation checks every id belongs to the threat model.

### D4. Validation at generation time and at accept time
Generation: parse into typed `TmContent`; check taxonomy values, ranges, ≥ 1 target, ids present; unknown **fields** such as `status`, `evidence`, `treatment`, `acceptance`, `thresholds`, `delete` ⇒ invalid (serde `deny_unknown_fields` on every payload type) — this implements "actions AI can never take" structurally, not by filtering. Accept: re-validate against current state (targets may have vanished) with the same domain functions as manual create; failure ⇒ 422, proposal stays pending. One bounded retry as in the editor.

### D5. What accept creates
Threat → status `open`, origin `ai`, `source` = model id (new `origin` value `ai` on `tm_threats`, `tm_controls`, `security_tests`, components/flows/assets/zones get an `origin` column default `manual`). Rule-based candidates keep their candidate step; an AI proposal already has its own staging, so a second confirmation would be noise. Rating accept writes one named rating on one threat. Control proposal may link an existing control instead of creating. Security tests are created as `draft` (as generated ones are), never `ready`. `tm-review` has no accept path (kind has no payload to apply).

### D6. Context rendering and limits
`TmContext` renders the architecture as compact JSON: zones (name, kind, trust), components (id, name, type, zone, owner, criticality, internetFacing, short description), flows (id, endpoints by name, relationship, protocol, auth, encrypted, crossing), assets (id, category, C/I/A, personalData) and, depending on kind, threats/controls with ratings and links. Descriptions are truncated to 400 chars. Size cap ~24k tokens estimated; if exceeded, reduce to targets + one hop of neighbours and flag `truncated`. `previewContext` returns exactly this JSON. Never included: provider keys, tenant ids, user emails.

### D7. Opt-in
`threat_models.ai_enabled boolean NOT NULL DEFAULT false`, changed by `PUT /threat-models/{id}` (role as for edit). Handlers check it before calling the provider. The enable dialog names provider and endpoint host from `GET /settings/ai`.

### D8. API shape (pending approval)
```
POST /threat-models/{tmId}/ai/proposals          {kind, prompt?, targetIds?, count?, previewContext?}  -> 202 Job | 200 context
GET  /threat-models/{tmId}/ai/proposals          ?status&kind
GET  /ai/tm-proposals/{id}                        (tenant scope resolves via threat model)
POST /ai/tm-proposals/{id}/accept                 {payload?}  optional edited payload
POST /ai/tm-proposals/{id}/reject                 {reason?}
PUT  /threat-models/{tmId}                        + aiEnabled
```
Rating/threat/etc. DTOs gain `origin` and `source` read fields. `GET /jobs/{id}` unchanged. New `TenantScope::TmProposal` (or addressed under the threat model to avoid a new scope — prefer nested `/threat-models/{tmId}/ai/proposals/{id}[/accept|/reject]`; decide at approval).

### D9. Frontend
`features/threat-model/components/ai-chat` modelled on the editor's `ai-chat-window` (client-side transcript, kind inference, polling, cards) but without `ModelSyncService`. Cards per kind: architecture card shows a diff-style list (new vs existing), threat card shows category/targets, rating card shows old vs proposed, review renders text with entity links. The store gets `proposals` and `aiEnabled`; selection from tables/diagram provides the focus id. `AiBadge` component used in tables, detail view and (if present) report.

### D10. Safety and operations
Rate limit per project reuses the editor's limiter with a shared counter. Logs: kind, prompt version, model id, sizes, outcome (extend the existing structured log). Prompt-injection test fixtures live next to the parser tests: malicious descriptions in context, answers with forbidden fields, ids from another threat model.

## Risks / Trade-offs

- [Architecture data leaves the deployment] → opt-in, provider/host notice, context preview, local Ollama supported by existing config.
- [LLM invents ids or entities] → id and taxonomy validation, retry with error, accept-time revalidation.
- [Noise / low-quality threats] → proposals are staged, rationale shown, bulk reject; counts not affected until accepted.
- [Prompt injection via model text] → delimited data, no tools, `deny_unknown_fields`, forbidden-action tests.
- [Separate proposal tables diverge] → shared job mechanism and shared UI patterns; ADR records the merge path.
- [Contract approval may reshape endpoints] → tasks start with the approval gate.
- [Small local models fail JSON] → retry once, clear error; kinds are small and schema-constrained.

## Migration Plan

Additive migration: `tm_proposals`, `threat_models.ai_enabled` (default false), `origin` (default `manual`) and `source` columns on architecture entities, and `ai` added to the threat/control/test origin CHECKs. Existing data is unaffected and AI stays off until enabled. Rollback drops the new objects.

## Open Questions

- Should enabling AI be restricted to an admin role rather than any editor? Default here: any editor.
- Should accepted AI threats be forced through the candidate step after all? Default here: no.
- Nested vs top-level proposal routes (D8) — decide when the contract is approved.
