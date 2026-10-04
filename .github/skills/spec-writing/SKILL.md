---
name: spec-writing
description: Writing and maintaining specifications, requirements and ADRs under docs/. Use when creating or changing requirements, API contracts, or architecture decisions.
---

# Spec writing

- One requirement per line, ID-prefixed: `FR-` functional, `NFR-` non-functional.
- Each requirement is testable; add acceptance criteria where non-obvious.
- Keep terminology consistent with `docs/specification/01-glossary.md`.
- `docs/specification/openapi.yaml` is the API source of truth; never change it (or `05-api.md`) without explicit user approval.
- For decisions use `docs/adr/0000-template.md`; number sequentially.
- Update cross-references when renaming or moving requirements.
