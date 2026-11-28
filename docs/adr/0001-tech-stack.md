---
title: 0001. Tech stack
type: adr
status: proposed
date: 2026-10-04
tags: [adr, stack, rust, angular]
related: [../architecture/overview.md]
---

# 0001. Tech stack

## Context
Need a performant backend for graph-based generation and a rich editor UI.

## Decision
- Backend: Rust, axum, tokio, sqlx with PostgreSQL (the first plan was SQLite; the project moved to PostgreSQL before release, see ADR 0002 for how it is deployed), serde, tracing.
- Frontend: Angular (standalone, signals); graph canvas library to be chosen in a later ADR.
- API: REST/JSON with OpenAPI.

## Consequences
Strong typing and performance on the backend; separate DTO layer needed; PostgreSQL needs a server, which `--dev-mode` starts in Docker for development.
