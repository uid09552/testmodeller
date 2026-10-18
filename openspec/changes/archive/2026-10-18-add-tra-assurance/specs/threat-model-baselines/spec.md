# Spec Delta

## Purpose

Freezes the threat model as it stood at a point in time so an assessment can be referred to later, and shows what changed since.

## ADDED Requirements

### Requirement: Create a baseline
The user SHALL be able to create a baseline of a threat model with a name (unique within the threat model) and an optional note. The baseline SHALL capture the scope fields, thresholds, all architecture entities, threats with ratings, treatments and acceptances, controls with evidence, security tests with latest results and the diagram layout, together with the creation time and the creator's name. A baseline SHALL be immutable: it cannot be edited, only deleted.

#### Scenario: Create
- **WHEN** the user creates the baseline "Release 2.3 review"
- **THEN** it appears in the baseline list with its creation time and counts of threats and controls

#### Scenario: Duplicate name
- **WHEN** the user creates a baseline whose name already exists
- **THEN** the creation is refused

#### Scenario: Live edits do not change it
- **WHEN** a threat is edited after the baseline was created
- **THEN** the baseline still shows the earlier value

### Requirement: View a baseline
A baseline SHALL open as a read-only view that shows the scope, the architecture (zones, components, data flows, trust-boundary crossings), the assets, the risk matrices, the risk register, the controls with their evidence and the open flags, visibly marked as a baseline with its name and date. No create, edit or delete action for model entities SHALL be offered in it. (The baseline view is the assessment report of the baseline; the diagram is not part of it.)

#### Scenario: Read-only
- **WHEN** the user opens a baseline
- **THEN** a banner shows its name and date and no editing control is offered

### Requirement: Compare with the live model
The user SHALL be able to compare a baseline with the live model. The comparison SHALL list threats added, removed or changed (status, level of inherent, current or residual risk, treatment), controls whose status changed, and totals per risk level for both sides. Matching SHALL be by entity id.

#### Scenario: Changed level
- **WHEN** a threat was High in the baseline and is Moderate now
- **THEN** the comparison lists it under changed with both levels

#### Scenario: Added and removed
- **WHEN** one threat was created and one deleted since the baseline
- **THEN** each appears under added or removed respectively

### Requirement: Delete a baseline
The user SHALL be able to delete a baseline after confirmation. Deleting the threat model or its project SHALL delete its baselines.

#### Scenario: Delete
- **WHEN** the user confirms deleting a baseline
- **THEN** it is removed from the list and its comparison is no longer available
