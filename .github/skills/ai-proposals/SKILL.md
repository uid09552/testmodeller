---
name: ai-proposals
description: AI-assisted scenario and test case proposals. Use when working on the ai crate, prompt templates, proposal review UI, or LLM provider integration.
---

# AI proposals

- Provider-agnostic trait in `crates/ai`; providers are adapters.
- AI returns structured JSON validated against the domain schema; invalid output is rejected, never repaired silently.
- Proposals are stored as `Proposal` with status `pending|accepted|rejected`; only accepted ones become domain entities.
- Treat model and user content as untrusted: no tool execution, no secrets in prompts, log prompt IDs not content.
- Details: `docs/specification/07-ai-integration.md`.
