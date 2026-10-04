# 2. Container deployment with nginx-hosted frontend

Date: 2026-10-04

## Status

Accepted

## Context

The backend and frontend had no deployment story: running the stack meant
`make dev` (a testcontainers-backed throwaway database) plus `ng serve` with a
dev proxy. That is fine for development but gives no reproducible artefact, and
the dev proxy (`frontend/proxy.conf.json`) only exists inside the Angular dev
server, so a built frontend had no way to reach the API.

We need images for both halves, something to host the built Angular bundle, and
a single command to bring the stack up.

## Decision

Three images, composed together:

- **Backend** — multi-stage Rust build on `rust:1.94-slim-bookworm`, runtime on
  `debian:bookworm-slim` as a non-root user (uid 10001). Dependencies compile in
  their own layer from manifests plus stub sources, so editing crate sources does
  not re-run the slow dependency build.
- **Frontend** — `node:24-alpine` builds the production bundle; `nginx:1.27-alpine`
  serves it. nginx also reverse-proxies `/api` to the backend.
- **PostgreSQL** — `postgres:16-alpine` with a named volume.

### nginx serves the frontend and proxies the API

The browser talks to one origin. This means no CORS configuration is needed on
the normal path, and it replaces `proxy.conf.json` (dev-server only) with
something that works for a built bundle.

### No `upstream` block; DNS resolved per request

nginx resolves `upstream` hostnames **once at startup** and refuses to boot if
they do not resolve. With an `upstream backend { server backend:8080; }` block, a
backend that is down or not yet created takes the whole frontend down with it —
confirmed in testing (`[emerg] host not found in upstream`).

Instead the config declares a `resolver` and puts a variable in `proxy_pass`,
which defers DNS to request time. nginx then starts regardless of backend state
and returns 502 for `/api` only, while continuing to serve the app. This also
means it picks up a backend's new IP after a restart.

`NGINX_ENTRYPOINT_LOCAL_RESOLVERS=1` is set because the nginx image's
`15-local-resolvers.envsh` returns early unless that flag is present, leaving
`NGINX_LOCAL_RESOLVERS` unset. The config is a `.template` so the image's
envsubst step fills it in; `NGINX_ENVSUBST_FILTER` restricts substitution to the
two variables we own.

### PostgreSQL is part of the compose stack

The backend requires `TM_DATABASE_URL` to start. Its `--dev-mode` flag starts a
throwaway database through testcontainers, which needs a Docker socket inside
the container — the wrong shape for a deployable stack. So compose runs a real
PostgreSQL service and the backend is never run with `--dev-mode` in a container.

Migrations are embedded at compile time via `sqlx::migrate!`, so the backend
applies them itself on startup; no separate migration step is needed. There are
no compile-time `query!` macros, so the image builds without a live database.

### Ports

| Service  | Host              | Why |
| -------- | ----------------- | --- |
| frontend | `8088`            | 8080 is taken by `make dev`, 4200 by `make dev-fe` |
| backend  | `127.0.0.1:8081`  | direct API access while developing; loopback only |
| db       | not published     | avoids clashing with a local PostgreSQL |

### Secrets

`POSTGRES_PASSWORD` is a required variable (`${POSTGRES_PASSWORD:?}`) so the
stack refuses to start rather than falling back to a default. `TM_AI_API_KEY` is
passed as runtime environment only — never baked into an image — which keeps the
in-memory-only guarantee from [ADR 0001](0001-tech-stack.md) and
`docs/specification/07-ai-integration.md` intact. `.env` is gitignored;
`.env.example` documents the variables.

## Consequences

- `docker compose up --build` brings up the whole stack; the app is on
  `http://localhost:8088`.
- The frontend image is static and environment-agnostic apart from
  `BACKEND_ORIGIN`, so the same image can point at a backend outside compose.
- Image sizes: backend ~160 MB, frontend ~74 MB.
- The backend image cannot use `--dev-mode`. Developers who want a throwaway
  database keep using `make dev` on the host.
- Rust dependency caching uses stub sources rather than `cargo-chef`; this adds
  no build-time tool but does mean the stub list in the Dockerfile must be
  updated when a crate is added to the workspace.
- `_sqlx_migrations` plus the 12 domain tables are created on first backend
  start, verified in testing.
