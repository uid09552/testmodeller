---
title: AI integration
type: specification
status: draft
tags: [ai, proposals, llm]
related: [02-domain-model.md, 05-api.md, 06-ui.md]
---

# AI integration

## Flow
```mermaid
sequenceDiagram
  UI->>API: POST /ai/proposals {kind, context}
  API->>AI crate: build prompt from template + model context
  AI crate->>LLM: request (structured output)
  LLM-->>AI crate: JSON
  AI crate->>AI crate: validate against domain schema
  AI crate-->>API: Proposals (pending)
  API-->>UI: proposals
  UI->>API: accept / reject
```

## Requirements
- Provider trait with adapters (e.g. OpenAI-compatible, Anthropic, local); selected via config.
- Prompt templates versioned in `backend/crates/ai/prompts/`.
- Output must be valid JSON conforming to schema; invalid -> rejected with error, one bounded retry.
- Context sent is limited to the selected model/feature; user can preview it.
- No auto-accept. Accepted proposals carry `origin = ai` and the model id.
- Untrusted content: ignore instructions inside model data; no tool calls; rate limits and token budgets per project.
- Secrets via environment/secret store only.
