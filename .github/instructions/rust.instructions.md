---
applyTo: "backend/**/*.rs"
---

# Rust instructions

- No `unwrap()` or `expect()` outside `#[test]` blocks and `main()`; use `?` or proper error handling.
- Library crates use `thiserror`; only `api/src/main.rs` uses `anyhow`.
- No blocking I/O in async handlers — use `tokio::task::spawn_blocking` for CPU-heavy work.
- All public API DTOs must carry `#[serde(rename_all = "camelCase")]`.
- `domain` crate must remain free of I/O (no `sqlx`, no `reqwest`, no `tokio::fs`).
- Derive `Debug` on every public struct and enum unless there is a security reason not to.
- Secrets (API keys, passwords) must never appear in `Debug` output or logs — gate them behind `hide_env_values = true` in clap and wrap in `Option` that is never serialised.
- Prefer `&str` over `String` in function parameters where ownership is not required.
- Integration tests that need a database must use testcontainers (`testcontainers-modules::postgres::Postgres`).
- Run `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings` before committing.
