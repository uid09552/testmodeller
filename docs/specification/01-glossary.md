---
title: Glossary
type: specification
status: draft
tags: [glossary, terminology]
related: [02-domain-model.md]
---

# Glossary

| Term | Definition |
| --- | --- |
| Project | Top-level container of all modelling artifacts |
| Component | Part of the system under test (e.g. "Payments") |
| Feature | Capability within a component; has a scenario description, models and test cases |
| Scenario | Only used as a feature's `scenarioDescription` (free text) |
| Model | State machine describing a feature's behavior |
| State | Node in a model |
| Transition | Edge with event, optional guard and action |
| Coverage criterion | Rule defining when generated tests are sufficient |
| Test case | Ordered steps with expected results derived from a model or written manually |
| Proposal | AI-suggested artifact awaiting user review |
