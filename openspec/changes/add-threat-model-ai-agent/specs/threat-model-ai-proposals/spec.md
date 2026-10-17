# Spec Delta

## Purpose

Lets an AI provider propose architecture, threats, ratings, controls and security tests for a threat model as pending proposals that the user reviews, edits, accepts or rejects.

## ADDED Requirements

### Requirement: Request proposals
The user SHALL be able to request proposals of a kind (`tm-architecture`, `tm-threats`, `tm-ratings`, `tm-controls`, `tm-security-tests`, `tm-review`) for a threat model with a free-text instruction (at most 8000 characters), an optional set of target entity ids and a count (1–20). The request SHALL run as an asynchronous job whose status is polled, and SHALL return pending proposals. A request SHALL be refused when no AI provider is configured or AI assistance is not enabled for the threat model.

#### Scenario: Request threats
- **WHEN** the user asks for `tm-threats` with a configured provider and AI enabled
- **THEN** a job is created and, when it succeeds, lists the created pending proposals

#### Scenario: No provider
- **WHEN** the provider is `none`
- **THEN** the request is refused with a message that no AI provider is configured

#### Scenario: AI not enabled
- **WHEN** the threat model has AI assistance disabled
- **THEN** the request is refused with a message saying how to enable it

### Requirement: Proposal content per kind
Each kind SHALL produce payloads that match the corresponding entity input. `tm-architecture`: zones, components, assets and flows, referring to each other by name within the proposal and to existing entities by id. `tm-threats`: STRIDE category, title, scenario, preconditions, consequences and at least one target (existing component or flow id). `tm-ratings`: for each given threat id, a likelihood and impact (1–5) for a named rating (inherent or current) and a rationale. `tm-controls`: control name, type, category, description, the threat ids it addresses and optionally an existing control id to link instead of creating one. `tm-security-tests`: title, steps with expected results, test type and the threat and control ids verified. `tm-review`: text findings only.

#### Scenario: Architecture from description
- **WHEN** the user describes "a web shop with a SPA, an API and a Postgres database behind a WAF"
- **THEN** pending proposals contain the components, zones and flows described, with names resolvable within the payload

#### Scenario: Ratings need existing threats
- **WHEN** a `tm-ratings` proposal names a threat id that does not exist in the threat model
- **THEN** that output is rejected as invalid

### Requirement: Validation and retry
Provider output SHALL be parsed and validated against domain rules before any proposal is stored: valid taxonomy values, unique names, at least one existing target per threat, ratings within 1–5, every referenced id present in the threat model. Invalid output SHALL trigger one retry with the validation error; a second failure SHALL fail the job with that error. Invalid output SHALL NOT be repaired silently or stored.

#### Scenario: Retry succeeds
- **WHEN** the first answer has a threat without a target and the second is valid
- **THEN** the valid proposals are stored

#### Scenario: Retry fails
- **WHEN** both answers are invalid
- **THEN** the job fails with the validation reason and no proposal is stored

### Requirement: Review proposals
Pending proposals SHALL be listable per threat model (filter by status and kind), viewable with their rationale and source model id, editable before accepting, and acceptable or rejectable individually. Accepting SHALL validate the (edited) payload with the same rules as a manual create and SHALL create the entities with `origin = ai` and the source model id; a refused accept SHALL leave the proposal pending. Rejecting SHALL record an optional reason and SHALL NOT create anything.

#### Scenario: Accept a threat
- **WHEN** the user accepts a pending `tm-threats` proposal
- **THEN** a threat with a display id, status open and origin AI exists, and the proposal is accepted with a reference to it

#### Scenario: Accept edited
- **WHEN** the user changes the title before accepting
- **THEN** the created threat has the edited title

#### Scenario: Accept refused
- **WHEN** a target component of the proposal was deleted since it was made
- **THEN** the accept is refused, the reason is shown and the proposal stays pending

#### Scenario: Reject
- **WHEN** the user rejects a proposal with a reason
- **THEN** nothing is created and the reason is stored

### Requirement: Ratings are applied only on accept
Accepting a `tm-ratings` proposal SHALL set the named rating of the named threat only; it SHALL NOT change any other rating, the treatment or the status. The proposal SHALL show the existing rating beside the proposed one.

#### Scenario: Apply rating
- **WHEN** the user accepts a proposed current rating 3 × 4 for a threat that had none
- **THEN** that threat's current rating is 3 × 4 and its other fields are unchanged

### Requirement: Provenance
Entities created from AI proposals SHALL show an AI marker with the source model id in lists, detail views and exports. Entities the user edits afterwards SHALL keep the marker. AI suggestions SHALL NOT be counted in Overview or matrix results until accepted.

#### Scenario: Marker
- **WHEN** an AI-created threat is listed
- **THEN** it shows an AI badge, and its tooltip names the model

#### Scenario: Pending not counted
- **WHEN** ten proposals are pending
- **THEN** the Overview and matrix counts do not include them
