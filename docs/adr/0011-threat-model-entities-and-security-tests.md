---
title: 0011. Threat models have their own entities, and security tests are a separate list
type: adr
status: accepted
date: 2026-10-17
tags: [domain, api, threat-modeling]
related: [../specification/09-threat-modeling.md, ../specification/02-domain-model.md, 0010-editor-layout-is-opaque.md]
---

# 0011. Threat models have their own entities, and security tests are a separate list

## Context

Threat modeling needs a graph of the architecture: services, databases, identity providers, external users, CI/CD pipelines, the flows between them, the zones they run in and the assets they handle. The project already has a *Component* (Project > Component > Feature > Model), but that is a unit of the test hierarchy — something features, models and test cases hang off. Most architecture elements (a database, an identity provider, an external user) would never have features or models.

Threats also need tests. Feature test cases are tied to models (assignments to states and transitions), coverage, traceability, staleness and result import. Security tests are derived from threats and controls, not from a state machine.

## Decision

- A threat model is a new aggregate, one per project. **Architecture components** are a new entity inside it, not an extension of `Component`. An architecture component may optionally link to one `Component` or `Feature` of the same project; the link is cleared when that entity is deleted, and nothing in the test hierarchy changes because of it.
- **Security tests** are a new entity owned by the threat model, not `TestCase`s. They do not take part in model assignments, coverage, traceability, staleness of generated model tests, project export or result import. Their results are recorded by hand; their CSV/Gherkin export runs in the browser.
- All analysis — trust-boundary crossings, warnings, risk levels, flags, rule matches, test templates, staleness — is pure domain code, recomputed on every read rather than stored.
- Child resources are addressed under `/threat-models/{id}/…`, so only the threat model is a new tenant scope; storage checks that an item belongs to the threat model in the path.
- The diagram layout is opaque to the backend, as in ADR 0010, but saved through its own endpoint with its own `layoutVersion`, so dragging shapes never conflicts with edits to the threat model's text.

## Consequences

- The glossary keeps "Component" unambiguous; the UI says "Component" only inside the threat modeling workspace.
- Security tests do not reach feature coverage or traceability. If that is wanted later, each generated test carries its template, threat and control, so they can be converted into feature test cases without loss.
- Rule suggestions need a memory of what was suggested: rule keys outlive their threats, so a dismissed or deleted suggestion is not suggested again.
- Each read loads the whole threat model. Threat models are small (hundreds of entities), so this fits NFR-002; caching by version is possible later.
