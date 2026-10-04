---
title: Workflow
type: guide
status: active
tags: [workflow, process]
related: [../adr/README.md]
---

# Workflow

1. Spec: update `docs/specification/` (and ADR if a decision is made).
2. Contract: update `05-api.md` if the API changes.
3. Implement backend and/or frontend with tests.
4. Run checks: `scripts/check.sh`.
5. Review (human or `reviewer` agent), then merge.

Branches: `feat/<id>-short-name`, `fix/...`. Commits: Conventional Commits (`feat:`, `fix:`, `docs:`).

## Working with agents
- Use `spec-writer` for docs, `backend-dev` / `frontend-dev` for code, `reviewer` before merge.
- Agents must cite the requirement ID they implement.
