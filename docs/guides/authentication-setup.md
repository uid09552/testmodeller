---
title: Configuring authentication
type: guide
status: active
tags: [auth, jwt, oidc, multi-tenancy, configuration]
related: [../specification/08-usermanagement.md, ../adr/0005-jwt-auth-and-tenancy.md]
---

# Configuring authentication

This guide covers connecting TestModeller to your identity provider: how to
tell the backend where tokens are signed, and where in the token it finds the
tenant and the role. For the rules behind it, see
[08 User management](../specification/08-usermanagement.md).

## How it fits together

```text
browser ──cookie──▶ APISIX gateway ──Authorization: Bearer <access token>──▶ backend
                     (OIDC handshake)                                  (validates the JWT,
                                                                        reads tenant + role)
```

- The **gateway** signs the user in with your identity provider and keeps the
  session in a cookie. It is configured by the `OIDC_*` variables.
- The **backend** checks the access token the gateway forwards. It is
  configured by the `TM_JWKS_URL`, `TM_*_CLAIM` and `TM_JWT_*` variables.

All of these go in `.env` (copy it from `.env.example`). Restart with
`make up` after changing them.

## 1. Register a client with your identity provider

Create an OIDC client (confidential, authorization code flow) with:

- redirect URI `http://localhost:8088/callback`, or your public URL followed by
  `/callback`;
- post-logout redirect URI `http://localhost:8088/`, or your public URL;
- an access token that carries the tenant (org id) and the roles. See
  [step 3](#3-tell-the-backend-where-the-tenant-and-role-are).

## 2. Configure the gateway and the key set

```bash
# Gateway: signs the user in
OIDC_CLIENT_ID=testmodeller
OIDC_CLIENT_SECRET=...
OIDC_DISCOVERY=https://idp.example.com/realms/testmodeller/.well-known/openid-configuration
OIDC_REALM=testmodeller
OIDC_REDIRECT_URI=http://localhost:8088/callback
OIDC_POST_LOGOUT_REDIRECT_URI=http://localhost:8088/
OIDC_SESSION_SECRET=$(openssl rand -hex 32)   # paste the generated value

# Backend: verifies the token's signature
TM_JWKS_URL=https://idp.example.com/realms/testmodeller/.well-known/openid-configuration
```

`TM_JWKS_URL` takes either the JWKS URL itself or the discovery document. A
URL containing `/.well-known/openid-configuration` is followed to its
`jwks_uri`. Only RS256/384/512 and ES256/384 signatures are accepted.

Optionally, pin the issuer and audience. Leave them empty to skip the check:

```bash
TM_JWT_ISSUER=https://idp.example.com/realms/testmodeller
TM_JWT_AUDIENCE=testmodeller
```

## 3. Tell the backend where the tenant and role are

Two settings name the claims to read:

| Variable | Default | Must resolve to |
| --- | --- | --- |
| `TM_TENANT_CLAIM` | `tenant` | A non-empty string, which becomes the tenant id |
| `TM_ROLE_CLAIM` | `roles` | A string, a space-separated string, or an array of strings |

Each value is a **claim path**. A plain name reads a top-level claim. Dots
reach into nested objects. Find your token's layout below.

### Top-level claims

```json
{ "sub": "u-1", "tenant": "acme", "roles": ["Editor"] }
```

```bash
TM_TENANT_CLAIM=tenant
TM_ROLE_CLAIM=roles
```

### Nested claims

```json
{ "sub": "u-1", "org": { "id": "acme" }, "realm_access": { "roles": ["editor", "offline_access"] } }
```

```bash
TM_TENANT_CLAIM=org.id
TM_ROLE_CLAIM=realm_access.roles
```

`realm_access.roles` is where Keycloak puts realm roles.

### Namespaced claims whose names contain dots

Some providers put their claims under a namespace that contains dots itself,
such as a domain name:

```json
{
  "sub": "u-1",
  "edge.siemens.cloud": { "tenant": "xyz", "roles": ["Editor"] }
}
```

Write the full path as is:

```bash
TM_TENANT_CLAIM=edge.siemens.cloud.tenant
TM_ROLE_CLAIM=edge.siemens.cloud.roles
```

No escaping is needed. At each level the backend picks the longest run of
segments that names a key that exists, so it finds the key
`edge.siemens.cloud` first and then `tenant` inside it. URL-style claim names
work the same way, e.g. `TM_ROLE_CLAIM=https://example.com/roles`.

In the rare case that a token has both a key `a.b` and a key `a` containing
`b`, the longer key `a.b` wins.

### Role names

The backend recognises two roles, case-insensitively:

| Role in the token | Access |
| --- | --- |
| `Editor` | Read and write |
| `User` | Read only (`GET`) |

Any other value is ignored, and a token with no recognised role is treated as
`User`. If your provider uses different names, such as `testmodeller-admin`,
map them to `Editor` / `User` in the provider. The backend has no role-name
mapping.

## 4. Check it

Start the stack with `make up`, sign in at <http://localhost:8088>, and open a
project. A request without a usable token fails with a problem response whose
`detail` says why. The token itself is never logged.

To see what your provider actually puts in a token, decode its payload
locally. Avoid pasting real tokens into online decoders.

```bash
TOKEN=eyJ...   # an access token from your provider
python3 -c 'import base64,json,sys; p=sys.argv[1].split(".")[1]; print(json.dumps(json.loads(base64.urlsafe_b64decode(p+"="*(-len(p)%4))),indent=2))' "$TOKEN"
```

Then call the backend directly with it:

```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/projects
```

The backend port is not published by `compose.yaml`. Run this against
`make dev` started without `--dev-mode`, or from inside the compose network.

## Troubleshooting

| Status | `detail` | Cause and fix |
| --- | --- | --- |
| `403` | the token carries no "…" claim, so it is not scoped to a tenant | `TM_TENANT_CLAIM` does not resolve to a non-empty string. Decode the token and check the path, including nesting. |
| `403` | the User role may not POST this resource | The role claim was not found, or holds no `Editor`. Check `TM_ROLE_CLAIM` and the role names. |
| `401` | the issuer is not accepted / the audience is not accepted | `TM_JWT_ISSUER` / `TM_JWT_AUDIENCE` do not match `iss` / `aud`. Fix them, or leave them empty. |
| `401` | the token was signed with a key the provider does not publish | `TM_JWKS_URL` points at a different realm or provider than the one issuing tokens. |
| `401` | token algorithm … is not accepted | The provider signs with HS256 or another unsupported algorithm. Switch the client to RS256 or ES256. |
| `503` | cannot reach the identity provider's key set | The backend container cannot reach `TM_JWKS_URL`. Check DNS and the network from inside the container. |

## Developing without an identity provider

`make dev` runs the backend with `--dev-mode`. It skips validation and treats
every request as `Editor` in tenant `dev`, so none of the above is needed. To
test claim paths against real tokens, run the backend without `--dev-mode`
instead:

```bash
cd backend && cargo run --bin testmodeller -- \
  --database-url postgres://... \
  --jwks-url https://idp.example.com/realms/testmodeller/.well-known/openid-configuration \
  --tenant-claim edge.siemens.cloud.tenant \
  --role-claim edge.siemens.cloud.roles
```

## Reference

| Flag / env | Default | Meaning |
| --- | --- | --- |
| `--jwks-url` / `TM_JWKS_URL` | — | JWKS or OIDC discovery URL. Required unless `--dev-mode`. |
| `--tenant-claim` / `TM_TENANT_CLAIM` | `tenant` | Claim path to the org id. |
| `--role-claim` / `TM_ROLE_CLAIM` | `roles` | Claim path to the role. |
| `--jwt-issuer` / `TM_JWT_ISSUER` | — | Expected `iss`; unchecked when unset. |
| `--jwt-audience` / `TM_JWT_AUDIENCE` | — | Expected `aud`; unchecked when unset. |
| `--jwks-cache-secs` / `TM_JWKS_CACHE_SECS` | `300` | How long a fetched key set is reused. |
| `--jwks-min-refresh-secs` / `TM_JWKS_MIN_REFRESH_SECS` | `30` | Minimum time between refreshes triggered by an unknown `kid`. |

`compose.yaml` passes the first five to the backend. The two cache settings
keep their defaults there unless you add them to the backend's `environment`.
