---
title: Test generation
type: specification
status: draft
tags: [generation, coverage, mbt]
related: [02-domain-model.md, 03-requirements.md]
---

# Test generation

## Coverage criteria
| Id | Description |
| --- | --- |
| `state` | Every state visited |
| `transition` | Every transition taken |
| `transition-pair` | Every pair of consecutive transitions |
| `bounded-paths` | All paths up to length N |

## Algorithm outline
1. Validate model; abort on errors.
2. Build directed graph.
3. Produce paths from initial state to final states (or to dead ends) covering the criterion, using a seeded traversal for tie-breaking.
4. Minimize: drop paths whose coverage is subsumed.
5. Convert each path to a TestCase with steps.

## Expressions
Guards/actions use a small expression language over model variables (booleans, integers, strings; `== != < > && || !`, assignment). Parsed in `domain`; infeasible paths (unsatisfiable guards) are skipped and reported.

## Output
Test cases plus a coverage report: covered/total per criterion, uncovered elements, skipped infeasible paths.

A saved generated test case is assigned to the start state (no step), and, for
every step `n`, to that step's transition and target state with `stepOrder = n`.
A transition the path takes more than once is assigned at each of its steps, so
the path can be rebuilt from the assignments.

## Stale tests
`GET /models/{modelId}/stale-tests` reports generated test cases of a model
whose path no longer fits the current model. It is derived on every call from
current data: nothing is stored, and nothing is changed, unassigned or deleted.

- Only test cases with `origin = generated` and `generatedFromModelId` equal to
  the model are checked. Manual and AI test cases (and the editor's per-state
  tests) have no path and are never reported.
- The path is rebuilt from the test case's transition assignments in this
  model, ordered by `stepOrder`, for steps `1..steps.length`. Test cases
  generated before every repeated transition was assigned may miss the
  assignment of a repeated step; such a step is filled with the one transition
  of the same test case that chains with its neighbours, if exactly one does.
- Reasons, with stable codes:
  - `STEP_UNASSIGNED`: a step has no transition, because it was deleted.
  - `STEPS_DISCONNECTED`: a step's transition no longer starts where the
    previous one ends (a transition was rewired).
  - `NOT_FROM_INITIAL`: the first step does not start at the initial state, or
    the model has no initial state.
  - `GUARD_UNSATISFIABLE`: replaying the path from the variables' initial
    values, a guard is false, or a guard or action cannot be evaluated.
- The guard check runs only on a complete, connected path. Because variables
  start at fixed initial values and actions are plain assignments, replaying
  the path decides satisfiability exactly: there is no search, no cap, and no
  "could not be proven" result.
- Limits: only the stored steps count, so a generated test case whose steps
  were edited by hand is checked against its edited step count; the check is
  one pass over the test case's assignments.
