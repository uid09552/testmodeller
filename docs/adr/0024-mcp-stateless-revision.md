---
title: 0024. MCP stateless revision 2026-07-28 and OAuth metadata
type: adr
status: accepted
date: 2026-11-28
tags: [mcp, protocol, oauth]
related: [0015-api-keys-and-mcp.md, 0018-upload-scoped-api-keys.md, ../specification/05-api.md]
---

# 0024. MCP stateless revision 2026-07-28 and OAuth metadata

## Context
MCP revision 2026-07-28 removes the `initialize` handshake, protocol sessions and
`ping`; every request names its version and capabilities, and the HTTP transport
adds headers and statuses. Clients on earlier revisions still exist (ADR 0015).
HTTP servers must also publish OAuth protected resource metadata (RFC 9728) when
they take OAuth tokens.

## Decision
- One endpoint serves both eras. A request whose `_meta` or `MCP-Protocol-Version`
  names `2026-07-28` is a stateless request; one that names an older supported
  version, or none, keeps the handshake behaviour. An unknown version is `400`
  with `-32022` and the supported versions.
- A stateless request must carry `MCP-Protocol-Version` equal to its `_meta`
  version, `Mcp-Method` equal to the method, and `Mcp-Name` equal to `params.name`
  for `tools/call` (else `400`, `-32020`), and `_meta` client capabilities (else
  `400`, `-32602`). `initialize` and `ping` are `404`. Results carry `resultType:
  "complete"`; `server/discover` and `tools/list` add `ttlMs` (60 s) and
  `cacheScope` (`private`: the endpoint is authenticated).
- `server/discover` is also answered for a client that sends no version.
- The server mints and reads no `Mcp-Session-Id`.
- OAuth: with `TM_MCP_OAUTH_ISSUER` and `TM_PUBLIC_URL` set, the metadata is
  served at `/.well-known/oauth-protected-resource` (and the path form) and a
  `401` from `/mcp` carries `WWW-Authenticate: Bearer resource_metadata="…"`. This
  is metadata only: `/mcp` still accepts API keys, not tokens of that server, so
  it is off by default. Validating tokens (audience per RFC 8707) is a later step.

## Consequences
- Not done: input-required (MRTR) approval of AI proposals, `Mcp-Param-*`
  headers, `subscriptions/listen`. `tm_decide_proposal` keeps its behaviour
  (the user chose "keep today's behaviour").
- The revision was read from its public text, not tested against a client that
  implements it; the first client to do so may need adjustments.
