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
| Threat model | A project's security model: architecture, threats, risks, controls and security tests (one per project) |
| Architecture component | Element of the architecture in a threat model (service, database, identity provider, external user…); not a *Component* of the test hierarchy, though it may link to one |
| Exposure zone | Where architecture components run, with a trust level 0 (untrusted) to 4 |
| Trust boundary | Crossed by a data flow whose endpoints are in zones of different trust level; computed, never entered |
| Data flow | Directed communication between two architecture components (relationship, protocol, authentication, encryption, assets) |
| Asset | Something worth protecting (data, secrets, code, capability), linked to components and flows |
| Threat | Potential attack scenario against components or flows, classified by STRIDE |
| Risk | Likelihood × impact of a threat, rated inherent (before controls) and current (with existing controls) |
| Security control | Measure that prevents, detects or corrects a threat |
| Security test | Test case of a threat model verifying a threat's controls; separate from feature test cases |
