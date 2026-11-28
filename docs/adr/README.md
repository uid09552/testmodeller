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
| [0011](0011-threat-model-entities-and-security-tests.md) | Threat models have their own entities, and security tests are a separate list | Accepted |
| [0012](0012-residual-risk-and-baselines.md) | Residual risk is entered by the analyst, and baselines are immutable snapshots | Accepted |
| [0013](0013-threat-model-proposals-store.md) | Threat model proposals have their own store and share the job mechanism | Accepted |
| [0014](0014-siemens-ix-design-system.md) | The UI uses the Siemens iX design system | Accepted |
| [0015](0015-api-keys-and-mcp.md) | Per-user API keys authenticate an in-process MCP endpoint | Accepted |
| [0016](0016-threats-covered-by-feature-test-cases.md) | Feature test cases can cover threats; security tests convert into them | Accepted |
| [0017](0017-more-generation-strategies.md) | Prime paths, reach-element and weighted random walks as generation criteria | Accepted |
| [0018](0018-upload-scoped-api-keys.md) | Upload-scoped API keys let CI post test results | Accepted |
| [0019](0019-ai-history-retention.md) | AI jobs and decided proposals are deleted after a retention period | Accepted |
| [0020](0020-playwright-export.md) | Playwright test skeletons as an export format | Accepted |
| [0021](0021-requirement-tags.md) | Requirement tags on states and transitions drive generation, coverage and traceability | Accepted |
| [0022](0022-sneak-path-tests.md) | Sneak-path tests for events a state does not allow | Accepted |
| [0023](0023-step-data.md) | Generated steps carry the variable values they start with | Accepted |
| [0024](0024-mcp-stateless-revision.md) | MCP stateless revision 2026-07-28 and OAuth metadata | Accepted |
