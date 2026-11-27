---
title: API
type: specification
status: draft
tags: [api, rest, contract]
related: [02-domain-model.md, 06-ui.md, 07-ai-integration.md]
---

# API

REST/JSON under `/api/v1`. Errors use RFC 7807 problem details. Updates require `If-Match: <version>`.

**The source of truth is [openapi.yaml](openapi.yaml). It must not be changed without explicit user approval.**

## Resource groups
| Tag | Scope |
| --- | --- |
| Projects | CRUD, explorer tree, search |
| Components | CRUD under a project, delete with `cascade` |
| Features | CRUD under a component, move, `scenarioDescription`, tags |
| Models | CRUD under a feature, atomic graph save (`PUT`, with the editor's opaque `layout`), duplicate, validate, versions, restore |
| States / Transitions | Fine-grained CRUD within a model (transition = model step) |
| TestCases | CRUD under a feature (manual), move, steps embedded, optional `implementationUrl` and `backlogUrl`, read-only `lastResult` |
| Assignments | Assign test cases to states/transitions; list per test case, state, transition, model |
| Generation | Generate test cases from a model (optionally saved and assigned); simulate one step of a model |
| Coverage | Per model, feature, component; stale generated test cases of a model |
| TestResults | Import JUnit XML / Cucumber JSON results for a project, result history per test case |
| Traceability | Read-only trace of backlog items to test cases, model elements and implementation links, with gaps |
| Export | JSON/CSV/Gherkin export, JSON import |
| AI | Async proposal jobs, list/accept/reject proposals |
| Settings | AI provider settings |
| ThreatModel | One threat model per project (`/projects/{id}/threat-model`); under `/threat-models/{id}`: scope and risk thresholds, diagram `layout` (opaque, own `layoutVersion`), summary, and CRUD of zones, architecture components, assets, data flows, threats, controls and security tests; suggest threats (STRIDE rules), bulk status, generate security tests, manual results. See [09-threat-modeling.md](09-threat-modeling.md) |
| ApiKeys | The caller's own API keys for the MCP endpoint: list, create (key shown once), revoke (`/me/api-keys`). See [08-usermanagement.md](08-usermanagement.md#api-keys) |
| Mcp | Model Context Protocol endpoint for AI clients such as VS Code (`POST /mcp`); see [MCP tools](#mcp-tools) |
| System | Health |

## MCP tools

`POST /api/v1/mcp` speaks MCP over Streamable HTTP: JSON-RPC 2.0 in the body, a
JSON response (no SSE stream; `GET` is `405`), protocol version `2025-06-18`.
It is authenticated with an API key (`Authorization: Bearer tm_…`), never a
JWT. Each tool is a task-level aggregate of the REST operations above and runs
them in-process as the key's tenant and role, so the tenant guard, the role rule,
validation and error mapping are the API's own. A failed operation becomes a tool
result with `isError: true` and the problem details as text; it is not a JSON-RPC
error. Tools that write (marked ✎) need a key with the `Editor` role.

| Tool | Arguments | Aggregates |
| --- | --- | --- |
| `list_projects` | – | `GET /projects` (all pages) and `GET /projects/{id}/tree` for each |
| `search` | `projectId`, `query`, `types?` | `GET /projects/{id}/search` |
| `get_model` | `modelId` | `GET /models/{id}`, `POST /models/{id}/validate`, `GET /models/{id}/coverage` |
| `save_model` ✎ | `featureId` (create) or `modelId` (replace), `name`, `states`, `transitions`, `description?` | `POST /features/{id}/models`, or `GET` + `PUT /models/{id}` with the current version |
| `generate_test_cases` ✎ | `modelId`, `criterion?` (default `transition`), `seed?`, `maxPathLength?`, `save?` | `POST /models/{id}/generate` |
| `list_test_cases` | `featureId` or `modelId` | `GET /features/{id}/test-cases` or `GET /models/{id}/test-cases` |
| `export` | `projectId`, `format` (`gherkin`, `csv`, `json`), `featureId?`, `componentId?` | `GET /projects/{id}/export` |
| `import_test_results` ✎ | `projectId`, `format` (`junit`, `cucumber`), `content`, `dryRun?` | `POST /projects/{id}/test-results` |
| `get_coverage` | one of `modelId`, `featureId`, `componentId` | `GET /{models,features,components}/{id}/coverage` |
| `get_traceability` | `projectId` | `GET /projects/{id}/traceability` |
| `get_threat_model` | `projectId` | `GET /projects/{id}/threat-model`, `…/summary`, `…/threats`, `…/controls` |
| `request_ai_proposals` ✎ | `featureId`, `kind`, `modelId?`, `prompt?`, `count?` | `POST /ai/proposals`, then polls `GET /jobs/{id}` for up to 30 s |
| `list_proposals` | `projectId`, `status?` | `GET /projects/{id}/proposals` |
| `decide_proposal` ✎ | `proposalId`, `decision` (`accept`, `reject`), `payload?` | `POST /proposals/{id}/accept` or `/reject` |

No tool accepts a proposal on its own: `request_ai_proposals` only creates
pending proposals, and `decide_proposal` is a separate, explicit call
(AGENTS.md rule 8).

## Conventions
- IDs are UUIDs; timestamps RFC 3339 UTC; fields camelCase.
- Pagination: `?limit=&cursor=`.
- AI endpoints may be slow: return `202` with a job resource, poll `/jobs/{id}`.
- Concurrency: `PUT`/`PATCH` require `If-Match`; mismatch returns `412`.
- Assignment targets must belong to a model of the test case's feature, otherwise `422`.
- Threat model collections return plain arrays (no paging). An item path whose item belongs to another threat model returns `404`. Link lists in an input (`assetIds`, `targets`, `controlIds`, …) replace the stored links as a whole; ids outside the threat model return `422`.
- Evidence (`/controls/{id}/evidence`, `/security-tests/{id}/results/{rid}/evidence`, `DELETE /evidence/{id}`) holds references only. Setting a control to `verified` without evidence returns `422`; deleting the last evidence of a verified control sets it to `implemented`. Baselines (`/baselines`, `/baselines/{id}`, `/baselines/{id}/compare`) are immutable; a duplicate name returns `409`, more than 50 per threat model `422`. Accepting a threat needs `acceptedBy`, `acceptanceRationale` and `acceptanceExpires` (after today, at most 3 years ahead), otherwise `422`.
- AI proposals of a threat model are addressed under it (`/threat-models/{id}/ai/proposals[/{pid}[/accept|/reject]]`); a request needs a configured provider (`503`) and `aiEnabled` on the threat model (`409`), is rate limited (`429`), runs as a job (`202`, poll `/jobs/{id}`) and with `previewContext` returns the context instead (`200`). Accepting validates against the current state and returns `422` with the proposal left pending. Read DTOs of AI-created entities carry `origin: ai` and `source` (the model id).
- Computed fields (a flow's `crossing`, `warnings`, a threat's scores, levels and `flags`, a security test's `stale` and `lastResult`) are derived on every read and ignored on input.
