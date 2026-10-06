---
title: User management
type: specification
status: draft
tags: [auth, jwt, multi-tenancy, rbac, gateway]
related: [03-requirements.md, 05-api.md, 06-ui.md, ../adr/0005-jwt-auth-and-tenancy.md]
---

# User management

Authentication happens at the edge. An APISIX gateway performs the OIDC
handshake with the identity provider and forwards requests to the UI and the
backend; the backend only validates the JWT it is handed. The SPA never sees a
token.

## Requirements

Source requirements, as given:

1. The backend validates JWT tokens unless `--dev-mode` is set.
2. The token is passed in the `Authorization` header.
3. The token contains a role and an org id.
4. The org id is the tenant and is used for tenant separation.
5. The claim name for the org id is configurable; the default is `tenant`. The
   org id and the role may sit in a nested claim, e.g.
   `"edge.siemens.cloud": { "tenant": "xyz", "roles": ["Editor"] }`.
6. Roles are `Editor` and `User`. `User` suffices for all `GET` requests; every
   mutating request (`POST`, `PUT`, `PATCH`, `DELETE`) requires `Editor`.
7. The JWKS well-known URL is configurable and is used to fetch the public key.
8. The user profile sits in the top-right corner and offers logout.
9. The deployment adds an APISIX container as the central gateway, performing
   the OIDC handshake and forwarding to the UI and the backend.

These map to FR-040 to FR-048 in [03-requirements.md](03-requirements.md).

## Token validation

- `Authorization: Bearer <jwt>`. A missing or malformed header is `401`.
- Signature verified against the JWKS fetched from the configured URL. Keys are
  cached; an unknown `kid` triggers at most one refresh per
  `TM_JWKS_MIN_REFRESH_SECS` so a rotating provider cannot be used to hammer the
  JWKS endpoint.
- `exp` and `nbf` are enforced. `iss` and `aud` are checked when configured.
- Supported algorithms: RS256/384/512 and ES256/384. `none` and the HMAC family
  are rejected, because a JWKS cannot carry a symmetric secret safely.
- The tenant claim (default `tenant`) must be a non-empty string. Missing or
  empty is `403` — a token without a tenant cannot be scoped to any data.
- The role claim (default `roles`) may be a string, a space-separated string or
  an array of strings. `Editor` wins over `User`; an unknown role is ignored.
  No recognised role means `User`.
- Both claim settings are paths. A plain name is a top-level claim; dots reach
  into nested objects (`org.id`). Claim names may contain dots themselves, so
  at each level the longest run of segments that names an existing key wins:
  `edge.siemens.cloud.tenant` resolves `{"edge.siemens.cloud": {"tenant": …}}`.
  A path that does not resolve counts as a missing claim.

## Dev mode

`--dev-mode` skips validation entirely and treats every request as the
`Editor` role in tenant `dev`. It is the only mode in which the backend runs
without a JWKS URL, and it already starts a throwaway database, so it never
touches real data.

## Tenant separation

`projects.tenant_id` carries the tenant. Everything else — components,
features, models, states, transitions, test cases, assignments, proposals — is
reachable only through a project, so a single column holds the whole boundary.

Enforcement has two parts:

- **Listing and creating projects** is filtered by, and stamped with, the
  caller's tenant.
- **Every other request** carries an entity id in its path. Middleware resolves
  that id to its owning project's tenant and compares it with the token's
  tenant. A mismatch is `404`, not `403`: whether an id exists in another
  tenant is not the caller's business.

`POST /ai/proposals` carries its feature in the body, so its handler performs
the same check on `featureId`. AI jobs are stamped with the tenant that started
them and `GET /jobs/{id}` is scoped to it.

`GET /health` needs no token.

## Roles

| Method | `User` | `Editor` |
| --- | --- | --- |
| `GET` | yes | yes |
| `POST`, `PUT`, `PATCH`, `DELETE` | `403` | yes |

The rule is applied by method, not per route, so a new endpoint is covered the
day it is added.

## Configuration

| Flag / env | Default | Meaning |
| --- | --- | --- |
| `--jwks-url` / `TM_JWKS_URL` | — | JWKS or OIDC discovery URL. Required unless `--dev-mode`. |
| `--tenant-claim` / `TM_TENANT_CLAIM` | `tenant` | Claim path to the org id, e.g. `edge.siemens.cloud.tenant`. |
| `--role-claim` / `TM_ROLE_CLAIM` | `roles` | Claim path to the role, e.g. `edge.siemens.cloud.roles`. |
| `--jwt-issuer` / `TM_JWT_ISSUER` | — | Expected `iss`; unchecked when unset. |
| `--jwt-audience` / `TM_JWT_AUDIENCE` | — | Expected `aud`; unchecked when unset. |
| `--jwks-cache-secs` / `TM_JWKS_CACHE_SECS` | `300` | How long a fetched key set is reused. |
| `--jwks-min-refresh-secs` / `TM_JWKS_MIN_REFRESH_SECS` | `30` | Floor between refreshes on an unknown `kid`. |

A discovery URL ending in `/.well-known/openid-configuration` is resolved to
its `jwks_uri` on first use.

For step-by-step setup with examples per token layout, see
[Configuring authentication](../guides/authentication-setup.md).

## UI

- The existing user menu in the top-right corner gains a **Sign out** item.
- Logout is a full navigation to the gateway's logout path (`/logout` by
  default, configurable at build time), because the session is a gateway
  cookie — clearing anything in the SPA would not end it.
- The SPA holds no token and adds no `Authorization` header. Requests go to the
  gateway on the same origin, and the gateway attaches the token.
- A `401` from the API means the gateway session expired; the UI reloads so the
  gateway can re-run the handshake. A `403` is surfaced as "your role does not
  allow this".

## Gateway

The compose stack gains an `apisix` service in standalone mode (no etcd) as the
only published port. Two routes:

- `/api/*` to the backend, with the `openid-connect` plugin in session mode and
  the access token forwarded in `Authorization`.
- everything else to the nginx container serving the SPA.

The OIDC client id, secret, discovery URL and session secret come from the
environment; none are baked into the image.
