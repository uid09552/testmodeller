# security-controls Specification

## Purpose
Records the security measures that prevent, detect or correct threats, their implementation status, and which threats each one addresses, so the user can see which threats are covered and which are not.

## Requirements

### Requirement: Control catalog
The user SHALL be able to create, edit and delete security controls in a threat model. A control has a display id (`CTL-<n>`, sequential, never reused), name (required), description, type (preventive, detective, corrective, compensating), category (identity and access, data protection, application and network, detection and response, secure development), implementation status (planned, implemented, verified), owner, acceptance criteria and the components it applies to.

#### Scenario: Create a control
- **WHEN** the user creates "Object-level authorization" as a preventive identity and access control, planned, applying to "Customer API"
- **THEN** it is listed on the Controls tab as `CTL-1` with those values

#### Scenario: Verified needs a date
- **WHEN** the user sets a control's status to verified
- **THEN** the date of verification is recorded and shown

### Requirement: Controls link to threats
A control SHALL be linkable to any number of threats and a threat to any number of controls. Deleting a control SHALL remove its links and leave the threats unchanged.

#### Scenario: Link from the threat
- **WHEN** the user links `CTL-1` to `THR-1` from the threat's editor
- **THEN** the control lists `THR-1` and the threat lists `CTL-1`

#### Scenario: Delete a linked control
- **WHEN** the user deletes a control linked to two threats
- **THEN** both threats remain with their ratings unchanged and no longer list the control

### Requirement: Recommended controls for a threat
A threat created by a STRIDE rule SHALL list the control recommendations of that rule (for example "service authentication" and "authorization at the receiving component" for an unauthenticated boundary crossing). The user SHALL be able to create a control from a recommendation, which creates it planned and links it to the threat.

#### Scenario: Create from recommendation
- **WHEN** the user chooses "Add as control" on a recommendation of a threat
- **THEN** a planned control with that name is created and linked to the threat

### Requirement: Coverage of threats by controls
The Controls tab SHALL show, for each control, its linked threats and linked security tests, and SHALL offer filters for controls without threats, controls without tests and controls by status. The Threats tab SHALL mark open threats that have no control, and open threats whose controls are all planned.

#### Scenario: Threat without a control
- **WHEN** an open threat has no linked control
- **THEN** the Threats tab marks it "No control"

#### Scenario: Only planned controls
- **WHEN** an open threat's only linked controls are planned
- **THEN** the Threats tab marks it "Controls planned only"

#### Scenario: Control without tests
- **WHEN** the user filters the Controls tab by "without tests"
- **THEN** only controls with no linked security test are listed
