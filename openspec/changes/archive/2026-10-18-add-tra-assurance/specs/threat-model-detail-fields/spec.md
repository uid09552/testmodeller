# Spec Delta

## Purpose

Adds the attributes a reviewer expects on threats, assets and flows, all optional so existing models stay valid.

## ADDED Requirements

### Requirement: Threat detail fields
A threat SHALL accept an optional threat source (free text, e.g. "external attacker"), detection opportunities, existing mitigations and a confidence (low, medium, high) in the assessment. The affected trust boundaries of a threat SHALL be shown, computed from its targets: the crossing flows it targets, or the crossing flows of its target component.

#### Scenario: Fields saved
- **WHEN** the user sets source "malicious tenant admin" and confidence "medium"
- **THEN** both are shown on the threat and in the register

#### Scenario: Computed boundary
- **WHEN** a threat targets a flow that crosses from zone Internet to zone DMZ
- **THEN** the threat shows that crossing as an affected trust boundary

### Requirement: Asset detail fields
An asset SHALL accept an optional classification (public, internal, confidential, restricted), data location, retention period and encryption at rest (yes, no, unknown; default unknown).

#### Scenario: Classification
- **WHEN** the user sets an asset to "restricted" with encryption at rest "no"
- **THEN** both are shown in the asset table

### Requirement: Flow detail fields
A flow SHALL accept an optional data classification (public, internal, confidential, restricted), authorization model (free text) and an entry-point flag marking it as an entry into the system from outside.

#### Scenario: Entry point
- **WHEN** the user marks a flow as an entry point
- **THEN** the flows table and the diagram mark it

### Requirement: Sensitive data crossing a boundary
A flow that carries an asset classified confidential or restricted, or an asset with personal data, and crosses a trust boundary towards a less trusted zone SHALL carry the warning "sensitive data towards less trusted zone". The warning SHALL be computed on read and never block a save.

#### Scenario: Warning
- **WHEN** a flow from zone Internal (trust 3) to zone Third-party (trust 1) carries a restricted asset
- **THEN** the flow shows the warning

#### Scenario: Towards a more trusted zone
- **WHEN** the same asset flows from trust 1 to trust 3
- **THEN** no such warning is shown
