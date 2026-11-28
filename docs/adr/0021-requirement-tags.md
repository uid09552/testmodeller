---
title: 0021. Requirement tags on states and transitions
type: adr
status: accepted
date: 2026-11-28
tags: [requirements, coverage, generation, traceability]
related: [0017-more-generation-strategies.md, ../specification/02-domain-model.md, ../specification/04-test-generation.md]
---

# 0021. Requirement tags on states and transitions

## Context
A requirement was only a test case's `backlogUrl`. That answers "which test
cases belong to this backlog item" but not "which part of the model implements
it, and is it covered", and generation could not aim at a requirement.

## Decision
- A state or transition carries `requirements[]`: backlog URLs or short keys
  such as `REQ-12`, at most 50, each at most 2048 characters. They are trimmed;
  blanks and duplicates are dropped. Storage is a `jsonb` array column, so
  stored models without the field stay valid (empty).
- Tags match as exact trimmed text. A URL tag therefore matches a test case's
  `backlogUrl` only when written the same way; URL normalisation is not applied
  to tags in this version.
- The `requirement` criterion produces one shortest path per distinct tag, to
  the nearest tagged element, like `reach-element`; a tag that cannot be reached
  is reported as a skipped path; identical paths are kept once.
- Coverage adds `requirements` (covered/total distinct tags of a model) and
  `uncoveredRequirements`. A tag is covered when some tagged element is covered.
  Aggregates over features and components add the per-model counts.
- Traceability adds `requirements[]`: the tag, its tagged elements and the test
  cases in scope that are assigned to one of them or link the tag as backlog
  URL; the summary counts `untestedRequirements`.
- `PATCH /states/{id}` and `PATCH /transitions/{id}` change `requirements` only
  when the list is not empty (like their other optional fields, absent keeps).
  Clearing tags goes through `PUT /models/{id}`.

## Consequences
- A random-walk stop condition by requirement coverage and AI-proposed tags are
  left out.
- Older importers drop the field.
