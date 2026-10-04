---
title: Architecture overview
type: architecture
status: draft
tags: [architecture, crates]
related: [../adr/0001-tech-stack.md]
---

# Architecture overview

## Backend crates

```mermaid
graph TD
  api --> domain
  api --> generation
  api --> storage
  api --> ai
  generation --> domain
  storage --> domain
  ai --> domain
```

| Crate | Responsibility |
| --- | --- |
| `domain` | Entities, validation, expression language. No I/O. |
| `generation` | Coverage-based test generation |
| `storage` | Repository traits + SQLite (sqlx) implementation |
| `ai` | Provider trait, prompts, response validation |
| `api` | axum HTTP server, DTOs, wiring, config |

## Frontend
Angular app: `core` (API client, auth/config), `shared` (UI kit), `features/{explorer,editor,test-cases,coverage,ai,settings}`.

## Cross-cutting
- Config via environment (`TM_` prefix) and optional config file.
- Logging/tracing via `tracing`.
- Migrations in `backend/crates/storage/migrations`.
