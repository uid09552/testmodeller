---
title: 0018. Upload-scoped API keys let CI post test results
type: adr
status: accepted
date: 2026-11-28
tags: [auth, ci, results, api]
related: [0015-api-keys-and-mcp.md, 0005-jwt-auth-and-tenancy.md, ../specification/08-usermanagement.md]
---

# 0018. Upload-scoped API keys let CI post test results

## Context
Test result import exists (JUnit XML, Cucumber JSON, matched by `@tm-<id>`
tags), but only through the UI: it needs a user session, and a CI pipeline has
none. ADR 0015 gave users API keys, accepted by `/mcp` only, so that a leaked
key could not use the REST API.

## Decision
- A key may be created for one project (`projectId` in `POST /me/api-keys`).
  This **upload key** is accepted by exactly one request:
  `POST /projects/{projectId}/test-results` of its own project. It acts as the
  creator's tenant with the `Editor` role, on that route only.
- Creating one needs the `Editor` role and a project of the creator's tenant
  (checked in the handler, rule 10). Other keys keep their meaning: MCP only.
- The check runs in an outer layer in front of the JWT layer, so the JWT path
  is unchanged. It accepts a `tm_` bearer only for the one route and project;
  everything else with a key is refused, and `/mcp` refuses upload keys.
  The layer hands the identity to the rest of the stack through a private
  request extension that cannot come from the wire.
- The gateway routes a request carrying `Authorization: Bearer tm_…` on that
  one path around the OIDC handshake; browser sessions are unaffected.
- Deleting the project deletes its upload keys.

## Consequences
- A leaked upload key can add test results to one project until it expires or
  is revoked, and nothing else; results are idempotent per run, so a replay
  adds nothing.
- The ten-key limit and the 90-day default expiry apply to upload keys too.
- A GitHub Action in the repository posts a result file with such a key.
