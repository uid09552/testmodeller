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
- Backend: Rust, axum, tokio, sqlx (SQLite initially, Postgres-ready), serde, tracing.
- Frontend: Angular (standalone, signals); graph canvas library to be chosen in a later ADR.
- API: REST/JSON with OpenAPI.

## Consequences
Strong typing and performance on the backend; separate DTO layer needed; SQLite limits concurrency until Postgres migration.
