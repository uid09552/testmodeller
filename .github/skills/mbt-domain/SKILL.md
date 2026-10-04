---
name: mbt-domain
description: Model-based testing domain knowledge for this project. Use when working on models, coverage criteria, or test case generation algorithms.
---

# MBT domain

- A model is a finite state machine: states, transitions (event, guard, action), initial/final states. A feature's scenario description is free text.
- Test generation walks the model to satisfy a coverage criterion: state, transition, transition-pair, all-paths-bounded.
- A test case is an ordered list of steps with expected results, traceable to the model and transitions covered; assigned to states/transitions.
- Generation must be deterministic for a given model, criterion and seed.
- Details: `docs/specification/02-domain-model.md` and `04-test-generation.md`.
