---
title: Architecture decision records
type: index
status: active
tags: [adr, index]
related: [0000-template.md]
---

# Architecture decision records

Use [0000-template.md](0000-template.md). Number sequentially.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-tech-stack.md) | Tech stack | Proposed |
| [0002](0002-container-deployment.md) | Container deployment with nginx-hosted frontend | Accepted |
| [0003](0003-ai-chat-on-proposal-endpoints.md) | AI chat panel runs on the existing proposal endpoints | Accepted |
| [0004](0004-explorer-tree-in-nav.md) | The explorer tree lives in the nav bar | Accepted |
| [0005](0005-jwt-auth-and-tenancy.md) | JWT at the edge, tenancy on the project row | Accepted |
| [0006](0006-ai-context-sync.md) | The AI panel syncs its own context to the backend | Superseded by 0008 |
| [0007](0007-ai-provider-from-environment.md) | The AI provider is configured in the environment | Accepted |
| [0008](0008-backend-is-the-only-store.md) | The backend is the only store of models | Accepted |
| [0009](0009-test-result-import.md) | Importing test results: matching by tag, parsing safely | Accepted |
| [0010](0010-editor-layout-is-opaque.md) | The editor's layout is stored with the model, opaque to the backend | Accepted |
