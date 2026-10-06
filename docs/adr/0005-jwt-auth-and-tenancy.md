---
title: 0005. JWT at the edge, tenancy on the project row
type: adr
status: accepted
date: 2026-10-05
tags: [auth, security, multi-tenancy]
related: [../specification/08-usermanagement.md]
---

# 0005. JWT at the edge, tenancy on the project row

## Context

The product has to become multi-tenant and authenticated. The requirements in
[08-usermanagement.md](../specification/08-usermanagement.md) fix some of the
design: an APISIX gateway does the OIDC handshake, the backend validates a JWT
from the `Authorization` header, the org id in the token is the tenant, and
roles are `User` (read) and `Editor` (write).

Two things were open: where the SPA gets its token, and how tenancy is enforced
across a schema of twelve tables and roughly a hundred queries.

## Decision

**The SPA never handles a token.** The gateway keeps the session in a cookie
and attaches the access token when it forwards to the backend. Logout is a
navigation to the gateway's logout path. Nothing token-shaped is stored in the
browser, so there is no refresh logic, no token in `localStorage`, and the SPA
code is unchanged apart from the logout item.

**Tenancy lives on `projects.tenant_id` alone.** Every other entity is
reachable only through a project, so one column expresses the whole boundary,
and it is enforced in two places instead of in every query:

- the project list and project creation are filtered by and stamped with the
  caller's tenant;
- middleware resolves the entity id in the request path to its owning project's
  tenant and rejects a mismatch before the handler runs.

A mismatch answers `404`, not `403`, so an id cannot be probed for existence in
another tenant.

**Roles are checked by HTTP method**, not per route: `GET` and `HEAD` need
`User`, everything else needs `Editor`.

## Consequences

- Adding a route gets tenancy and role checks for free, as long as its path
  carries the entity id. A route that identifies its subject in the body —
  today only `POST /ai/proposals` — must check in the handler, and the
  middleware fails closed for any path shape it does not recognise.
- No per-query `tenant_id = $n` predicates to forget, and no change to the
  existing queries.
- The guard costs one small indexed lookup per request. Its join chain is at
  most four levels deep (transition → model → feature → component → project).
- The tenant and role claims are dotted paths, because providers often nest
  them under a namespaced claim (`"edge.siemens.cloud": {"tenant": …}`). Claim
  names may contain dots, so each level takes the longest matching key rather
  than requiring an escape syntax (JSON Pointer was the alternative; it is
  unambiguous but awkward to write in an environment variable).
- A tenant cannot be renamed by moving rows; the tenant id is whatever the
  identity provider puts in the claim, and it is a text column for that reason.
- Row-level security was the alternative. It would enforce in the database, but
  it needs the tenant set per transaction on a pooled connection, which means
  every query goes through a transaction wrapper. That is a larger change with
  more ways to get silently wrong than one middleware.
