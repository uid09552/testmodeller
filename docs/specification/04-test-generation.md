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
Guards/actions use a small expression language over scenario variables (booleans, integers, strings; `== != < > && || !`, assignment). Parsed in `domain`; infeasible paths (unsatisfiable guards) are skipped and reported.

## Output
Test cases plus a coverage report: covered/total per criterion, uncovered elements, skipped infeasible paths.
