# AGENTS.md

Guidance for AI agents working in this repository.

## Project

TestModeller: model-based testing tool. See [docs/specification/00-overview.md](docs/specification/00-overview.md).

## Rules

1. Spec first: read the relevant file in `docs/specification/` before changing code. If the spec is wrong or missing, update it in the same change.
2. Record significant technical decisions as an ADR in `docs/adr/`.
3. Follow the guides: [Rust](docs/guides/rust-coding-guide.md), [Angular](docs/guides/angular-coding-guide.md), [Testing](docs/guides/testing-guide.md), [Workflow](docs/guides/workflow.md).
4. Keep changes small and scoped; do not refactor unrelated code.
5. Backend and frontend communicate only through the HTTP API defined in `docs/specification/openapi.yaml` (summary in `05-api.md`). Change the contract first, then both sides.
6. NEVER change the API spec (`openapi.yaml`, `05-api.md`) without explicit user approval. Propose the change, wait for approval, then apply it. Implementation must conform to the spec as written.
7. Terminology: Project > Component > Feature > Model. "Scenario" refers only to a feature's `scenarioDescription`.
8. AI-generated output (proposals) is never persisted as accepted data without explicit user approval.

## Where things live

| Area | Path |
| --- | --- |
| Domain model, pure logic | `backend/crates/domain` |
| Test generation algorithms | `backend/crates/generation` |
| LLM integration | `backend/crates/ai` |
| Persistence | `backend/crates/storage` |
| HTTP API (binary) | `backend/crates/api` |
| Angular app | `frontend/` |

## Custom agents and skills

- Agents: `.github/agents/`
- Skills: `.github/skills/` (rust-development, angular-development, mbt-domain, ai-proposals, spec-writing)
- Prompts: `.github/prompts/`

## Definition of done

- Spec and docs updated
- Tests added/updated and passing
- `cargo fmt`, `cargo clippy -D warnings`, `ng lint` clean
