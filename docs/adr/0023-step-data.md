---
title: 0023. Generated steps carry the variable values they start with
type: adr
status: accepted
date: 2026-11-28
tags: [generation, test-data]
related: [0017-more-generation-strategies.md, ../specification/04-test-generation.md]
---

# 0023. Generated steps carry the variable values they start with

## Context
Guards and actions read model variables, but a generated test case said only
"login [when attempts > 2]". The person running it had to work out the values.
Parameters with partitions, boundary values and pairwise combinations (issue
#43) are larger steps; this is the first one.

## Decision
- A test step has an optional `data` object: the values of all model variables
  when the step starts, computed by replaying the path's actions on the
  variables' initial values. It is stored with the step (the steps column is
  jsonb, so there is no migration) and accepted on input, so editing a
  generated test case with the steps as read keeps it.
- No `data` for a model without variables, or from the step on whose action
  cannot be replayed.
- A sneak-path step carries the values at the end of its path.
- The stale-test check ignores `data`.

## Consequences
- Variables still have only a fixed initial value: choosing other values, input
  domains, boundaries and pairwise combinations are later slices of #43.
- The canvas editor keeps `data` per step while the number of steps is
  unchanged; adding or removing a step drops it.
