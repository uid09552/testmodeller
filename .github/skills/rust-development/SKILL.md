---
name: rust-development
description: Rust backend development for TestModeller. Use when adding or changing code in backend/, crates, API handlers, domain types, or tests.
---

# Rust development

1. Read `docs/guides/rust-coding-guide.md` and the relevant spec.
2. Crate boundaries: `domain` (pure) <- `generation`, `storage`, `ai` <- `api`. No upward dependencies.
3. Errors: `thiserror` in libraries, `anyhow` only in `api` main.
4. Async: tokio; no blocking calls in handlers; use `spawn_blocking` for CPU work.
5. Serialization: serde with explicit `#[serde(rename_all = "camelCase")]` on API DTOs.
6. Database: sqlx 0.8 + PostgreSQL. Migrations in `storage/migrations/` (never edit existing files — add a new one). Wrap multi-step writes in a transaction passed as `&mut PgConnection`.
7. Pagination: keyset `(created_at, id)` cursor only — no OFFSET.
8. Concurrency: optimistic via `version` integer; check and increment on every mutable operation.
9. Dev mode: `--dev-mode` flag starts a throwaway Postgres via testcontainers; `TM_DATABASE_URL` overrides in production.
10. Secrets: never log API keys or prompt content. Use `hide_env_values = true` in clap args that hold secrets.
11. No `unwrap()`/`expect()` outside `#[test]` blocks and `main()`.
12. Verify: `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`.
