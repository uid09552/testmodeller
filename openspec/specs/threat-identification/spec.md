# threat-identification Specification

## Purpose
Records what can go wrong as structured threats linked to the architecture, and suggests candidate threats by applying STRIDE rules to the registered components, flows, zones and assets, with each suggestion explained and confirmed by a person.

## Requirements

### Requirement: Threat records
The user SHALL be able to create, edit and delete threats. A threat has a display id (`THR-<n>`, sequential per threat model, never reused), title (required), STRIDE category (spoofing, tampering, repudiation, information disclosure, denial of service, elevation of privilege), scenario description, attacker objective, preconditions, consequences, at least one target (architecture component or data flow), linked assets, status and origin (manual or rule).

#### Scenario: Create a manual threat
- **WHEN** the user creates threat "Unauthorized access to customer records" of category information disclosure targeting the "Customer API" component
- **THEN** it is saved with id `THR-1`, origin manual and status open

#### Scenario: Threat without a target
- **WHEN** a client creates a threat with no component or flow target
- **THEN** the request is rejected as invalid

#### Scenario: Ids are not reused
- **WHEN** threat `THR-2` is deleted and a new threat is created
- **THEN** the new threat's id is `THR-3`

### Requirement: Threat lifecycle
A threat's status SHALL be one of candidate, open, mitigated, accepted or dismissed. A candidate SHALL become open only by a user confirming it; dismissing SHALL require a reason; accepted SHALL require the risk treatment "accept" with a rationale (see threat-risk-assessment).

#### Scenario: Dismiss without reason
- **WHEN** the user dismisses a threat without entering a reason
- **THEN** the change is refused and the reason field is marked required

#### Scenario: Confirm a candidate
- **WHEN** the user confirms a candidate threat
- **THEN** its status becomes open and it is counted on the Overview

### Requirement: STRIDE rule-based candidates
"Suggest threats" on the Threats tab SHALL apply a fixed, versioned set of STRIDE rules to the current architecture and create candidate threats for every rule match that does not already have a threat from the same rule on the same target. Candidates SHALL NOT count as open threats. The rules SHALL include at least those listed in the scenarios.

#### Scenario: Unauthenticated boundary crossing
- **WHEN** a flow crosses a trust boundary into a more trusted zone and its authentication is "none" or "unknown"
- **THEN** a spoofing candidate is created on that flow

#### Scenario: Unencrypted crossing with sensitive data
- **WHEN** a flow crosses a trust boundary, its encryption is "no" or "unknown", and a linked asset has high confidentiality
- **THEN** an information disclosure candidate is created on that flow

#### Scenario: Internet-facing component with sensitive asset
- **WHEN** an internet-facing component is linked to an asset with high confidentiality
- **THEN** an information disclosure candidate ("unauthorized data access") is created on that component

#### Scenario: Data store with high-integrity asset
- **WHEN** a data component is linked to an asset with high integrity requirement
- **THEN** a tampering candidate is created on that component

#### Scenario: Identity provider
- **WHEN** a component of type identity provider is the target of "authenticates through" flows from two or more components
- **THEN** a spoofing candidate ("identity provider compromise affects dependants") is created on it, naming the dependants

#### Scenario: CI/CD pipeline with deploy flow
- **WHEN** a CI/CD pipeline component has a "deploys" flow to any component
- **THEN** a tampering candidate ("build or deployment integrity") is created on the pipeline

#### Scenario: Internet-facing component availability
- **WHEN** a component is internet-facing
- **THEN** a denial of service candidate is created on it

#### Scenario: Missing audit
- **WHEN** a component has high criticality and no "sends data" flow to a component of type audit service
- **THEN** a repudiation candidate is created on it

#### Scenario: No duplicates on re-run
- **WHEN** the user runs "Suggest threats" twice without changing the architecture
- **THEN** the second run creates no new candidates and says so

#### Scenario: Dismissed candidates stay dismissed
- **WHEN** a candidate from a rule on a target was dismissed and the user runs "Suggest threats" again
- **THEN** no new candidate is created for that rule and target

#### Scenario: Deleted rule threats are remembered
- **WHEN** a threat created by a rule on a target was deleted and the user runs "Suggest threats" again
- **THEN** no new candidate is created for that rule and target

### Requirement: Candidates explain themselves
Every rule-created threat SHALL record the rule id and version, the facts that matched (entity names and attribute values), the information that would change the result (for example "document authentication on this flow"), and a reminder that undocumented controls are a question to check, not a finding.

#### Scenario: Explanation shown
- **WHEN** the user opens a spoofing candidate created on flow "Web app → Customer API"
- **THEN** the threat shows the rule's name, "crosses Internet (0) → Internal (3)", "authentication: unknown", and "document the authentication method to confirm or dismiss"

### Requirement: Threat list
The Threats tab SHALL list threats with id, title, category, targets, status, origin and current risk level, sortable and filterable by status, category, target, origin and risk level. Candidates SHALL be visually and textually distinguished from open threats.

#### Scenario: Filter candidates
- **WHEN** the user filters by status "candidate"
- **THEN** only candidates are listed, each labelled "Candidate"

#### Scenario: Bulk confirm
- **WHEN** the user selects three candidates and chooses "Confirm"
- **THEN** all three become open in one action
