---
title: 0015. Per-user API keys authenticate an in-process MCP endpoint
type: adr
status: accepted
date: 2026-11-27
tags: [auth, mcp, ai, api]
related: [../specification/08-usermanagement.md, ../specification/05-api.md, 0005-jwt-auth-and-tenancy.md]
---

# 0015. Per-user API keys authenticate an in-process MCP endpoint

## Context

Users want AI clients such as VS Code to work with their models, test cases
and threat models through the Model Context Protocol. Those clients cannot do
the gateway's browser OIDC handshake (ADR 0005), and the REST API has about a
hundred fine-grained operations, too many and too low-level to be useful tools.

## Decision

- **Per-user API keys.** `/me/api-keys` creates, lists and revokes keys. A key
  is `tm_` + 256 random bits, shown once and stored as a SHA-256 hash (the key
  is random, so a slow password hash adds nothing). It records the creator's
  `sub`, tenant and role, expires (90 days by default, 365 at most) and can be
  revoked. A key is accepted **only** by `POST /api/v1/mcp`.
- **MCP in the backend binary**, at `/api/v1/mcp`, over Streamable HTTP with
  JSON responses (no SSE). The protocol lives in a small `tm-mcp` crate (JSON-RPC
  framing, tool catalogue, argument checks) with no web or storage dependency;
  the API crate supplies the transport and authentication.
- **Tools are aggregates that dispatch in-process through the `/api/v1`
  router.** Each call runs as the key's identity through the same tenant guard,
  role rule, validation and error mapping as REST. The alternative, calling
  storage directly, would duplicate those checks in a second place where they
  could drift.
- `/me/*` is exempt from the role-by-method rule: managing one's own keys is
  not a write to project data.
- The gateway routes `/api/v1/mcp` without `openid-connect`.

## Consequences

- The MCP tool surface cannot exceed what the key's role and tenant allow over
  REST, and a new REST check protects MCP the day it is added.
- A key keeps the role it was created with until it expires or is revoked, even
  if the user's role is later reduced. Expiry bounds that window; revoking is
  immediate.
- One more credential type to protect: keys are hashed at rest, never logged,
  and useless outside `/mcp`.
- The tool catalogue in 05-api.md is part of the contract and changes with the
  same approval as the OpenAPI file.
