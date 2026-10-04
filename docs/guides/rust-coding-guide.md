---
title: Rust coding guide
type: guide
status: active
tags: [rust, backend, conventions]
related: [testing-guide.md, workflow.md]
---

# Rust coding guide

- Edition 2021+, stable toolchain pinned in `backend/rust-toolchain.toml`.
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- Errors: `thiserror` enums per crate; `anyhow` only in the `api` binary entry.
- No `unwrap`/`expect`/`panic!` in non-test code unless an invariant is documented.
- Prefer newtypes for IDs; derive `Debug, Clone, Serialize, Deserialize` deliberately.
- Traits at crate boundaries (`Repository`, `LlmProvider`); concrete types inside.
- Async only where I/O occurs; never block inside async handlers.
- Public items documented with a one-line doc comment.
- API DTOs are separate from domain types; map explicitly.
- Dependencies: justify additions; prefer well-maintained crates; run `cargo audit` in CI.
- Tests: unit tests in-module, integration tests in `tests/`, property tests (`proptest`) for generation.
