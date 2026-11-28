---
title: 0022. Sneak-path tests for events a state does not allow
type: adr
status: accepted
date: 2026-11-28
tags: [generation, negative-testing]
related: [0017-more-generation-strategies.md, ../specification/04-test-generation.md]
---

# 0022. Sneak-path tests for events a state does not allow

## Context
Every criterion so far follows transitions the model has. Faults often hide in
what the model does not allow: an event arriving in a state that has no
transition for it.

## Decision
- The *event alphabet* of a model is its distinct, non-empty transition
  `event` values. An event is *not allowed* in a non-final state when no
  transition leaving that state carries it. Guards are not considered: a
  guarded transition allows its event (a guard-blocked variant is left out).
- Criterion `sneak-path`: for each non-final state, in model order, and each
  disallowed event, in order of first appearance, one test: the shortest
  feasible path from the initial state to the state, then one extra step whose
  action is the event and whose expected result is that the event is rejected
  and the system stays in the state. Deterministic, no seed needed.
- The extra step has no transition, so it is not assigned to any element, and
  the stale-test check leaves it out (a test case tagged `sneak-path`).
- States that cannot be reached are skipped and reported once. At most 1000
  tests; the limit is reported as a skipped path.
- Sneak tests join no minimisation: each pair is its own test.

## Consequences
- The number of tests grows with states times events; the limit bounds it.
- What "rejected" means in the system under test is for the person
  implementing the test to decide; the expected text is generic.
