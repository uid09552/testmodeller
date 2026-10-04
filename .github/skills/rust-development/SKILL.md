---
name: rust-development
description: Rust backend development for TestModeller. Use when adding or changing code in backend/, crates, API handlers, domain types, or tests.
---

# Rust development

1. Read `docs/guides/rust-coding-guide.md` and the relevant spec.
2. Crate boundaries: `domain` (pure) <- `generation`, `storage`, `ai` <- `api`. No upward dependencies.
3. Errors: `thiserror` in libraries, `anyhow` only in `api` main.
4. Async: tokio; no blocking calls in handlers.
5. Serialization: serde with explicit `#[serde(rename_all = "camelCase")]` on API DTOs.
6. Verify: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
