---
title: Testing guide
type: guide
status: active
tags: [testing, quality]
related: [rust-coding-guide.md, angular-coding-guide.md]
---

# Testing guide

| Level | Backend | Frontend |
| --- | --- | --- |
| Unit | `cargo test` in-module | component/service tests |
| Property | `proptest` for generation (coverage achieved, determinism) | n/a |
| Integration | `backend/crates/api/tests` with in-memory SQLite | HTTP mocked |
| E2E | Playwright against running stack | `frontend/e2e` |

- Every requirement (`FR-*`) maps to at least one test; reference the ID in the test name or comment.
- AI tests use a fake `LlmProvider`; never call real providers in CI.
- Bug fixes start with a failing test.
