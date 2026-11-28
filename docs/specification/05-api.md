---
title: API
type: specification
status: draft
tags: [api, rest, contract]
related: [02-domain-model.md, 06-ui.md, 07-ai-integration.md]
---

# API

REST/JSON under `/api/v1`. Errors use RFC 7807 problem details. Updates of versioned entities require `If-Match: <version>` (exceptions below).

**The source of truth is [openapi.yaml](openapi.yaml). It must not be changed without explicit user approval.**

## Resource groups
| Tag | Scope |
| --- | --- |
| Projects | CRUD, explorer tree, search |
| Components | CRUD under a project, delete with `cascade` |
| Features | CRUD under a component, move, `scenarioDescription`, tags |
| Models | CRUD under a feature, atomic graph save (`PUT`, with the editor's opaque `layout`), duplicate, validate, versions, restore |
| States / Transitions | Fine-grained CRUD within a model (transition = model step) |
| TestCases | CRUD under a feature (manual), move, steps embedded, optional `implementationUrl` and `backlogUrl`, `threatIds` (threats of the project's threat model it covers), read-only `lastResult` |
| Assignments | Assign test cases to states/transitions; list per test case, state, transition, model |
| Generation | Generate test cases from a model (optionally saved and assigned) by criterion: `state`, `transition`, `transition-pair`, `bounded-paths`, `prime-paths`, `reach-element` (shortest path to `targetId`) or a seeded weighted `random` walk with stop conditions (`stopAfterTests`, `stopAtCoverage`); simulate one step of a model |
| Coverage | Per model, feature, component; stale generated test cases of a model |
| TestResults | Import JUnit XML / Cucumber JSON results for a project, result history per test case |
| Traceability | Read-only trace of backlog items to test cases, model elements and implementation links, with gaps |
| Export | JSON/CSV/Gherkin/Playwright export, JSON import |
| AI | Async proposal jobs, list/accept/reject proposals |
| Settings | AI provider settings |
| ThreatModel | One threat model per project (`/projects/{id}/threat-model`); under `/threat-models/{id}`: scope and risk thresholds, diagram `layout` (opaque, own `layoutVersion`), summary, and CRUD of zones, architecture components, assets, data flows, threats, controls and security tests; suggest threats (STRIDE rules), bulk status, generate security tests, manual results, convert a security test into a feature test case. See [09-threat-modeling.md](09-threat-modeling.md) |
| ApiKeys | The caller's own API keys for the MCP endpoint, or upload-only keys for one project's test results (`projectId`): list, create (key shown once), revoke (`/me/api-keys`). See [08-usermanagement.md](08-usermanagement.md#api-keys) |
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
| `tm_list_projects` | `limit?`, `detail?` | `GET /projects` (pages up to `limit`) and `GET /projects/{id}/tree` for each |
| `tm_search` | `projectId`, `query`, `types?`, `limit?` | `GET /projects/{id}/search` |
| `tm_get_model` | `modelId` | `GET /models/{id}`, `POST /models/{id}/validate`, `GET /models/{id}/coverage` |
| `tm_save_model` ✎ | `featureId` (create) or `modelId` (replace), `name`, `states`, `transitions`, `description?` | `POST /features/{id}/models`, or `GET` + `PUT /models/{id}` with the current version |
| `tm_generate_test_cases` ✎ | `modelId`, `criterion?` (default `transition`), `seed?`, `maxPathLength?`, `save?` | `POST /models/{id}/generate` |
| `tm_list_test_cases` | `featureId` or `modelId`, `limit?`, `detail?` | `GET /features/{id}/test-cases` or `GET /models/{id}/test-cases` |
| `tm_export` | `projectId`, `format` (`gherkin`, `csv`, `json`, `playwright`), `featureId?`, `componentId?` | `GET /projects/{id}/export` |
| `tm_import_test_results` ✎ | `projectId`, `format` (`junit`, `cucumber`), `content`, `dryRun?` | `POST /projects/{id}/test-results` |
| `tm_get_coverage` | one of `modelId`, `featureId`, `componentId` | `GET /{models,features,components}/{id}/coverage` |
| `tm_get_traceability` | `projectId` | `GET /projects/{id}/traceability` |
| `tm_get_threat_model` | `projectId` | `GET /projects/{id}/threat-model`, `…/summary`, `…/threats`, `…/controls` |
| `tm_request_ai_proposals` ✎ | `featureId`, `kind`, `modelId?`, `prompt?`, `count?` | `POST /ai/proposals`, then polls `GET /jobs/{id}` for up to 30 s; still running, it returns the `jobId` |
| `tm_get_ai_job` | `jobId` | `GET /jobs/{id}`, and `GET /proposals/{id}` for each proposal of a succeeded job |
| `tm_list_proposals` | `projectId`, `status?`, `limit?`, `detail?` | `GET /projects/{id}/proposals` |
| `tm_decide_proposal` ✎ | `proposalId`, `decision` (`accept`, `reject`), `payload?` | `POST /proposals/{id}/accept` or `/reject` |
| `tm_get_project_status` | `projectId` | `GET /projects/{id}/tree`, per model `GET /models/{id}`, `POST /models/{id}/validate` and `GET /models/{id}/coverage` (at most 50 models), `GET /projects/{id}/traceability` and `GET /projects/{id}/proposals?status=pending` |
| `tm_triage_failures` | `projectId`, `limit?` | `GET /projects/{id}/traceability` and `GET /models/{id}`: failed or errored last results grouped by the state or transition their test case is assigned to, most failures first; failures with no assignment are listed apart |
| `tm_update_test_case` ✎ | `testCaseId`, any of `name`, `description`, `preconditions`, `priority`, `status`, `tags`, `implementationUrl`, `backlogUrl`, `steps` | `GET` + `PUT /test-cases/{id}` with the current version; omitted fields, assignments and threat links are kept |
| `tm_patch_model` ✎ | `modelId`, `name?`, `description?`, `addStates?`, `addTransitions?`, `removeStateIds?`, `removeTransitionIds?` | `GET` + `PUT /models/{id}` with the current version; removing a state removes its transitions |

Names carry the `tm_` prefix so they do not collide with other servers' tools
(`search`, `export`). Listings take `limit` (1–500, default 100) and report
`truncated` when more exist; `detail` is `concise` (default: ids, names,
status, latest result) or `full` (every field), to keep results within
clients' tool-output limits. Every tool declares an `outputSchema` and the MCP
tool annotations (`readOnlyHint`, `destructiveHint`, `idempotentHint`).

No tool accepts a proposal on its own: `tm_request_ai_proposals` and
`tm_get_ai_job` only return pending proposals, and `tm_decide_proposal` is a
separate, explicit call
(AGENTS.md rule 8).

## Conventions
- IDs are UUIDs; timestamps RFC 3339 UTC; fields camelCase.
- Pagination: `?limit=&cursor=`.
- AI endpoints may be slow: return `202` with a job resource, poll `/jobs/{id}`. `POST /ai/proposals` is rate limited per project and per tenant (`429`, see [07-ai-integration.md](07-ai-integration.md)).
- Concurrency: `PUT`/`PATCH` of a versioned entity require `If-Match`; mismatch returns `412`. Exceptions, which take no `If-Match`:
  - `PATCH /states/{id}` and `PATCH /transitions/{id}`: fine-grained edits of one element; the model's own version still guards `PUT /models/{id}`.
  - `PUT /test-cases/{id}/assignments`, `PUT /states/{id}/test-cases/{tcId}` and `PUT /transitions/{id}/test-cases/{tcId}`: assignment links, which carry no version; the assign and unassign calls are idempotent.
  - `PUT /settings/ai`: a singleton without a version; the last write wins.
- Assignment targets must belong to a model of the test case's feature, otherwise `422`.
- Threat model collections return plain arrays (no paging). An item path whose item belongs to another threat model returns `404`. Link lists in an input (`assetIds`, `targets`, `controlIds`, …) replace the stored links as a whole; ids outside the threat model return `422`.
- Evidence (`/controls/{id}/evidence`, `/security-tests/{id}/results/{rid}/evidence`, `DELETE /evidence/{id}`) holds references only. Setting a control to `verified` without evidence returns `422`; deleting the last evidence of a verified control sets it to `implemented`. Baselines (`/baselines`, `/baselines/{id}`, `/baselines/{id}/compare`) are immutable; a duplicate name returns `409`, more than 50 per threat model `422`. Accepting a threat needs `acceptedBy`, `acceptanceRationale` and `acceptanceExpires` (after today, at most 3 years ahead), otherwise `422`.
- AI proposals of a threat model are addressed under it (`/threat-models/{id}/ai/proposals[/{pid}[/accept|/reject]]`); a request needs a configured provider (`503`) and `aiEnabled` on the threat model (`409`), is rate limited (`429`), runs as a job (`202`, poll `/jobs/{id}`) and with `previewContext` returns the context instead (`200`). Accepting validates against the current state and returns `422` with the proposal left pending. Read DTOs of AI-created entities carry `origin: ai` and `source` (the model id).
- Computed fields (a flow's `crossing`, `warnings`, a threat's scores, levels, `flags` and `testCaseIds`, a security test's `stale`, `lastResult` and `convertedToTestCaseId`) are derived on every read and ignored on input.
- Threats and feature test cases are linked from the test case side (`threatIds` on `TestCaseInput`; a threat of another project is `422`). `POST /threat-models/{id}/security-tests/{sid}/convert` with `{featureId, modelId?}` copies a security test into a new manual test case of that feature, linked to its threats (and, with `modelId`, assigned to that model's initial state), and sets the security test obsolete (`201`; already converted `409`; a feature of another project `422`).
