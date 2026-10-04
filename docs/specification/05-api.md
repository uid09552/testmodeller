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
| Models | CRUD under a feature, atomic graph save (`PUT`), duplicate, validate, versions, restore |
| States / Transitions | Fine-grained CRUD within a model (transition = model step) |
| TestCases | CRUD under a feature (manual), move, steps embedded |
| Assignments | Assign test cases to states/transitions; list per test case, state, transition, model |
| Generation | Generate test cases from a model (optionally saved and assigned) |
| Coverage | Per model, feature, component |
| Export | JSON/CSV/Gherkin export, JSON import |
| AI | Async proposal jobs, list/accept/reject proposals |
| Settings | AI provider settings |
| System | Health |

## Conventions
- IDs are UUIDs; timestamps RFC 3339 UTC; fields camelCase.
- Pagination: `?limit=&cursor=`.
- AI endpoints may be slow: return `202` with a job resource, poll `/jobs/{id}`.
- Concurrency: `PUT`/`PATCH` require `If-Match`; mismatch returns `412`.
- Assignment targets must belong to a model of the test case's feature, otherwise `422`.
