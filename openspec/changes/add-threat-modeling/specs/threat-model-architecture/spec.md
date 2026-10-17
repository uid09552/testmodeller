# Spec Delta

## Purpose

Lets the user register what is being built — architecture components, the data flows between them, the exposure zones they run in and the assets they handle — as a connected graph from which trust-boundary crossings are computed.

## ADDED Requirements

### Requirement: Register architecture components
The user SHALL be able to create, edit and delete architecture components in a threat model. Each has a name (required, unique within the threat model), description, type, owner, technology, criticality (low, medium, high, critical), internet-facing flag, zone (optional) and an optional link to one existing Component or Feature of the same project.

#### Scenario: Register a component
- **WHEN** the user adds a component "Customer API" of type REST/GraphQL API, internet-facing, in zone "DMZ"
- **THEN** it is listed on the Components tab with those values and appears on the diagram

#### Scenario: Duplicate name
- **WHEN** the user names a component like an existing one in the same threat model
- **THEN** the save is rejected with a message naming the clash

#### Scenario: Link to a test component
- **WHEN** the user links the architecture component to the existing Component "Payments"
- **THEN** the link is stored and shown, and the existing Component is unchanged

#### Scenario: Link across projects rejected
- **WHEN** a client links an architecture component to a Component of another project
- **THEN** the request is rejected as invalid

### Requirement: Component type taxonomy
The component type SHALL be one of a fixed list of types, each belonging to one layer: client, integration, logic, data, identity and security, platform, or external entity. The editor SHALL offer the types grouped by layer.

#### Scenario: Types per layer
- **WHEN** the user opens the type selector
- **THEN** it offers client (web frontend, mobile app, desktop client, browser extension), integration (API gateway, REST/GraphQL API, message broker, webhook, external connector), logic (service, worker, scheduler, serverless function), data (relational database, object storage, cache, search index, file store, backup), identity and security (identity provider, authorization service, secrets manager, key management, audit service), platform (cloud service, container platform, CI/CD pipeline, third-party SaaS, library) and external entity (human user, external system)

#### Scenario: Unknown type
- **WHEN** a client sends a component with type "mainframe"
- **THEN** the request is rejected as invalid and names the allowed types

### Requirement: Exposure zones
The user SHALL be able to create, edit and delete exposure zones, each with a name (unique within the threat model), description, kind (internet, partner, DMZ, internal, restricted data, management, identity, CI/CD, third party, other) and trust level from 0 (untrusted) to 4 (most trusted). Deleting a zone SHALL leave its components without a zone.

#### Scenario: Create a zone
- **WHEN** the user creates zone "Internet" of kind internet with trust level 0
- **THEN** the zone is listed on the Zones tab and drawn on the diagram

#### Scenario: Delete a zone in use
- **WHEN** the user deletes a zone that holds two components
- **THEN** the zone is removed and both components have no zone

### Requirement: Assets as independent entities
The user SHALL be able to create, edit and delete assets, each with a name (unique within the threat model), category (business data, identity and secrets, software and code, infrastructure, security asset, operational asset, business capability), owner, confidentiality, integrity and availability requirement (each low, medium, high) and a personal-data flag. An asset SHALL be linkable to any number of components and data flows.

#### Scenario: Shared asset
- **WHEN** the asset "Customer records" is linked to the "Customer API", "Reporting service" and "Backup job" components
- **THEN** the asset lists all three, and each component lists the asset

#### Scenario: Delete a linked asset
- **WHEN** the user deletes an asset linked to components, flows and threats
- **THEN** the asset and its links are removed and the components, flows and threats remain

### Requirement: Relationships and data flows
The user SHALL be able to create, edit and delete data flows between two architecture components of the same threat model. A flow has a source, target, relationship type (calls, sends data, reads/writes, authenticates through, trusts, depends on, runs on, publishes/subscribes, replicates/backs up, deploys), label, protocol, bidirectional flag, authentication (none, unknown, or a free-text method), encrypted-in-transit (yes, no, unknown) and linked assets. Several flows MAY connect the same pair.

#### Scenario: Two flows between the same components
- **WHEN** the user adds an HTTPS "calls" flow and an AMQP "publishes" flow from "Order service" to "Broker"
- **THEN** both flows are stored and drawn as separate arrows

#### Scenario: Flow to a component of another threat model
- **WHEN** a client creates a flow whose target belongs to a different threat model
- **THEN** the request is rejected as invalid

#### Scenario: Delete a component with flows
- **WHEN** the user deletes a component that is the source or target of flows
- **THEN** the user is told how many flows will be deleted, and on confirmation the component and those flows are removed

### Requirement: Computed trust-boundary crossings
A data flow SHALL be reported as crossing a trust boundary when its source and target are in different zones with different trust levels, or when exactly one endpoint has no zone. The crossing, its direction (into more or less trusted) and both trust levels SHALL be computed by the system on every read, never stored as user input.

#### Scenario: Internet to internal
- **WHEN** a flow goes from a component in zone "Internet" (trust 0) to one in zone "Internal" (trust 3)
- **THEN** the flow is marked as crossing a boundary into a more trusted zone

#### Scenario: Same trust level
- **WHEN** a flow connects components in two different zones that both have trust level 3
- **THEN** the flow is not marked as crossing

#### Scenario: Zone change updates crossings
- **WHEN** the user moves a component into a zone with another trust level
- **THEN** every flow touching it shows its crossing status recomputed without further action

### Requirement: Architecture tables
The Components, Data Flows, Zones and Assets tabs SHALL each list their entities in a sortable, filterable table with an editor for one entity at a time, and every action SHALL be reachable by keyboard.

#### Scenario: Filter flows that cross boundaries
- **WHEN** the user filters the Data Flows tab to "crosses trust boundary"
- **THEN** only crossing flows are listed

#### Scenario: Keyboard editing
- **WHEN** the user focuses a row and presses Enter
- **THEN** that entity's editor opens with focus on its first field

### Requirement: Missing-information warnings
The architecture SHALL be checked for gaps and each gap shown as a warning on the entity and on the Overview: a component with no zone, an internet-facing component with no flow, a flow with unknown authentication or encryption that crosses a trust boundary, an asset linked to nothing, and a component with no owner.

#### Scenario: Crossing flow with unknown encryption
- **WHEN** a flow crosses a trust boundary and its encryption is "unknown"
- **THEN** the flow carries a warning "encryption in transit not documented" and the Overview counts it

### Requirement: Optional mapping of components to features
An architecture component MAY be mapped to one Feature of the project (created in the project tree) and to one Component of the project. The mapping SHALL be optional, offered by choosing from the project's own features grouped by their component, shown in the Components table, and SHALL never be required to register or analyse a component. Choosing a feature SHALL also map the feature's component when none is mapped.

#### Scenario: Map a component to a feature
- **WHEN** the user picks the feature "Pay" of the project component "Payments" in a component's editor
- **THEN** the component is mapped to that feature and to "Payments", and the table shows "Payments › Pay"

#### Scenario: Unmapped is fine
- **WHEN** a component has no feature
- **THEN** it is registered and analysed like any other, and the table shows a dash
