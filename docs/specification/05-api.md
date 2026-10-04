---
title: API
type: specification
status: draft
tags: [api, rest, contract]
related: [02-domain-model.md, 06-ui.md, 07-ai-integration.md]
---

# API

REST/JSON under `/api/v1`. Errors use RFC 7807 problem details. Updates require `If-Match: <version>`.

## Resources
| Method | Path | Purpose |
| --- | --- | --- |
| GET/POST | `/projects` | List / create |
| GET/PATCH/DELETE | `/projects/{id}` | |
| GET/POST | `/projects/{id}/components` | |
| GET/POST | `/components/{id}/features` | |
| GET/POST | `/features/{id}/scenarios` | |
| GET/PUT/DELETE | `/scenarios/{id}` | Includes states and transitions |
| POST | `/scenarios/{id}/validate` | Model validation issues |
| GET | `/scenarios/{id}/versions` | History |
| POST | `/scenarios/{id}/generate` | Body: `{criterion, seed?}` -> test cases + coverage |
| GET | `/scenarios/{id}/test-cases` | |
| GET | `/features/{id}/coverage`, `/components/{id}/coverage` | Aggregated coverage |
| GET | `/projects/{id}/export?format=json\|csv\|gherkin` | |
| POST | `/ai/proposals` | Body: `{kind, context}` -> pending proposals |
| GET | `/projects/{id}/proposals?status=` | |
| POST | `/proposals/{id}/accept` / `reject` | Accept may include edited payload |

## Conventions
- IDs are UUIDs; timestamps RFC 3339 UTC; fields camelCase.
- Pagination: `?limit=&cursor=`.
- AI endpoints may be slow: return `202` with a job resource, poll `/jobs/{id}`.

A machine-readable `openapi.yaml` will live in `docs/specification/openapi.yaml` once the first endpoints are defined.
