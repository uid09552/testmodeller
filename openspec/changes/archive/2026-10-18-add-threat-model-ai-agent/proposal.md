# Proposal

## Why

The model editor has an AI assistant (chat panel → proposal jobs → cards the user accepts or rejects, ADR 0003/0007, `07-ai-integration.md`). The threat modeling workspace has none, although it is where a model is most tedious to build: turning an architecture description into components and flows, brainstorming threats beyond the eight fixed STRIDE rules, rating risks and finding controls. The same provider, review-before-persist discipline and safety rules should apply, so analysts get the assistant everywhere in the tool without a second AI stack.

## What Changes

- **AI chat panel in the threat model workspace**, opened from the workspace header, scoped to the current threat model, modelled on the editor's panel: a client-side transcript, one message → one proposal job → proposal cards, per-card Accept / Edit / Reject. The `kind` is inferred from the message unless picked.
- **Proposal kinds** for a threat model: `tm-architecture` (zones, components, flows, assets from a free-text description), `tm-threats` (additional threats grounded in the existing architecture, STRIDE-categorised, with targets), `tm-ratings` (likelihood/impact with rationale for chosen threats), `tm-controls` (controls for a threat, linkable to existing ones), `tm-security-tests` (tailored tests for a threat/control), and `tm-review` (advisory gap review in text; nothing to accept).
- **Grounded and validated output**: proposals reference existing entities by id, unknown ids and violations of the domain rules (unique names, ≥ 1 target, rating range, taxonomy values) are rejected with one bounded retry, exactly like the editor's proposals.
- **Nothing is persisted as accepted data without approval** (AGENTS.md rule 8): proposals live in their own pending list; accepting creates the entity through the normal domain validation with `origin = ai` and the model id. AI can never set a control to verified, add evidence, accept a risk, dismiss a threat or change thresholds.
- **Visible provenance**: AI-created entities carry an "AI" badge in tables, the diagram detail view and the report; AI suggestions are never mixed into computed results.
- **Privacy safeguards**: AI assistance is off per threat model until enabled; a context preview shows what will be sent; the context is limited to this threat model and size-capped; user-entered text in the model is treated as data, not instructions.
- **Same provider, same configuration**: reuses `TM_AI_*` env configuration, provider adapters, token budget, retry and prompt versioning from the `ai` crate; adds `tm` prompt templates and a threat-model context renderer.
- **API**: new proposal endpoints under `/threat-models/{tmId}/ai/…` (contract change — requires explicit approval before `openapi.yaml` / `05-api.md` are edited, AGENTS.md rule 6). Job polling reuses `GET /jobs/{id}`.
- **Docs**: extend `07-ai-integration.md`, `09-threat-modeling.md`, `06-ui.md`, requirements (FR-08x); ADR for "threat model proposals are a separate store, same job mechanism".

### Assumptions (to confirm at review)
- The threat model already lives in the backend, so unlike ADR 0006 no context sync is needed.
- Accepting an AI-proposed threat creates it as **open** (accept is the user's confirmation); rule-based candidates keep their own confirm step.
- No tool calling or autonomous agent loop: one request → validated proposals. "Agent" here means the chat assistant, as in the model editor.
- Conversation history is not sent to the provider (as ADR 0003); follow-ups are fresh instructions.

### Not in this change
Multi-turn history, tool use / autonomous actions, diagram-image or document ingestion, AI attack paths, AI-written reports, natural-language query over the model, duplicate-threat consolidation, per-organisation prompt libraries.

## Capabilities

### New Capabilities
- `threat-model-ai-proposals`: proposal kinds, grounding and validation, pending list, accept/edit/reject, provenance.
- `threat-model-ai-chat`: the workspace chat panel, kind inference, context preview, transcript and error states.
- `threat-model-ai-safeguards`: opt-in per threat model, context limits, untrusted-content handling, forbidden AI actions.

### Modified Capabilities
None in `openspec/specs/`. The base threat-model capabilities live in the unarchived `add-threat-modeling` change; the editor's AI behaviour is documented in `07-ai-integration.md` rather than a spec and is not changed.

## Impact

- **Backend**: `ai` crate (tm context renderer, 6 prompt templates `tm-*.v1.md`, tm parsers/validators), `domain::threat` (proposal payload types and apply-validation reuse), `storage` (migration `0009_tm_ai.sql` — or next free number: `tm_proposals`, `threat_models.ai_enabled`, `origin`/`source` on AI-created rows), `api` (handlers, DTOs, job runner reuse).
- **Frontend**: `features/threat-model/components/ai-chat`, store additions, badges, settings toggle.
- **Contract**: `openapi.yaml`, `05-api.md` — requires user approval.
- **Docs**: `07-ai-integration.md`, `09-threat-modeling.md`, `06-ui.md`, `03-requirements.md`, one ADR.
- Independent of `add-tra-assurance`; if both land, the report marks AI-origin items.
