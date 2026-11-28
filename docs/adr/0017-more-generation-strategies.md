---
title: 0017. Prime paths, reach-element and weighted random walks as generation criteria
type: adr
status: accepted
date: 2026-11-28
tags: [generation, coverage, api]
related: [../specification/04-test-generation.md, ../specification/05-api.md]
---

# 0017. Prime paths, reach-element and weighted random walks as generation criteria

## Context
Generation offered four criteria (state, transition, transition-pair,
bounded-paths). Other model-based testing tools also offer a criterion that
covers loops without enumerating every path (prime paths), a way to ask for
one specific scenario (shortest path to an element), and random walks driven
by transition probabilities with stop conditions (GraphWalker, AltWalker,
MaTeLo).

## Decision
- Three criteria are added to `CoverageCriterion`: `prime-paths`,
  `reach-element` (with `targetId`) and `random` (with `stopAfterTests` and
  `stopAtCoverage`).
- A transition gets an optional `weight` (0–1000, default 1) that only the
  `random` criterion reads. Weight 0 means never taken.
- All stay deterministic for model, options and seed (FR-021): the random walk
  uses the existing seeded generator, not system randomness.
- Prime paths are computed on the state graph and then checked against guards
  by walking them; a prime path whose guards do not hold in that order is
  reported as skipped, not searched for by other routes. At most 1 000 prime
  paths and 1 000 random walks are produced, with a note when cut off.
- Existing requests and stored models are unaffected: new fields are optional.

## Consequences
- The editor must carry `weight` through when it saves a model, or a weighted
  model set through the API would lose its weights; it does, and offers a
  field for it.
- Random walks are not minimized: each walk is a test case, so a stop
  condition is always applied.
- The MCP `tm_generate_test_cases` tool still offers the original four
  criteria; extending it is a change to the MCP tool table and waits for
  approval.
