# Spec Delta

## Purpose

Gives each project a threat model and a dedicated workspace, reached from a "Threat Modeling" navigation item, where the user scopes the model and sees a summary of its architecture, threats, risks, controls and security tests.

## ADDED Requirements

### Requirement: Threat model in the project tree
The project tree SHALL list a project's threat model as an item directly under the project, with a shield icon of its own and the threat model's name. The navigation bar SHALL have no "Threat Modeling" item. Selecting the item SHALL open the workspace for that project, and the project SHALL be in the URL.

#### Scenario: Open from the tree
- **WHEN** the user clicks the threat model item under project "Shop"
- **THEN** the workspace opens for "Shop" and the URL names that project

#### Scenario: No threat model, no item
- **WHEN** a project has no threat model
- **THEN** no threat model item is listed under it

#### Scenario: Deep link
- **WHEN** the user opens a workspace URL naming project P
- **THEN** the workspace shows P's threat model

### Requirement: Create and delete from the tree
The right-click menu of a project without a threat model SHALL offer "New Threat Model", which creates it (named after the project) and opens it. A project with one SHALL offer "Open Threat Model" instead. The threat model item's menu SHALL offer "Open Threat Model" and "Delete Threat Model", which asks for confirmation because everything in it is deleted.

#### Scenario: New Threat Model
- **WHEN** the user right-clicks project "Blog", which has none, and chooses "New Threat Model"
- **THEN** a threat model "Blog threat model" is created, listed under "Blog" and opened

#### Scenario: Delete asks first
- **WHEN** the user chooses "Delete Threat Model" and declines the confirmation
- **THEN** nothing is deleted

### Requirement: One threat model per project
Each project SHALL have at most one threat model. Opening the workspace for a project without one SHALL offer "Create threat model" and SHALL NOT create it implicitly. Deleting a project SHALL delete its threat model and everything in it.

#### Scenario: No threat model yet
- **WHEN** the user opens the workspace for a project with no threat model
- **THEN** the workspace shows an empty state with a "Create threat model" action and nothing is created

#### Scenario: Create
- **WHEN** the user chooses "Create threat model"
- **THEN** an empty threat model is created for the project and the Overview tab is shown

#### Scenario: Second creation rejected
- **WHEN** a client asks to create a threat model for a project that already has one
- **THEN** the request is rejected as a conflict and the existing model is unchanged

### Requirement: Threat model scope
A threat model SHALL carry a name, scope, objectives, assumptions and exclusions as free text (markdown), editable on the Overview tab and saved with optimistic concurrency.

#### Scenario: Edit assumptions
- **WHEN** the user edits the assumptions and saves
- **THEN** the new text is shown after a reload

#### Scenario: Concurrent edit
- **WHEN** the user saves scope text after another user has saved the same threat model
- **THEN** the save is rejected as a version conflict and the user is told to reload

### Requirement: Workspace tabs
The workspace SHALL offer tabs Overview, Diagram, Components, Data Flows, Zones, Assets, Threats, Risks, Controls and Security Tests. The active tab SHALL be in the URL so a link reopens it.

#### Scenario: Link to a tab
- **WHEN** the user opens a workspace URL whose tab is Threats
- **THEN** the Threats tab is active

#### Scenario: Keyboard
- **WHEN** the tab list has focus and the user presses the arrow keys
- **THEN** focus moves between tabs and Enter activates the focused tab

### Requirement: Overview summary
The Overview tab SHALL show the number of architecture components, assets, data flows and trust-boundary crossings; threats by status; open threats by risk level; the 5 × 5 matrix with the count of open threats per cell; the share of controls that are verified; the open threats without a linked control; and the controls without a linked security test.

#### Scenario: Counts reflect the model
- **WHEN** the threat model has 3 open threats, 1 of them rated Critical, and 1 open threat with no control
- **THEN** the Overview shows 3 open threats, 1 Critical, and lists the threat without a control

#### Scenario: Gap lists link to the item
- **WHEN** the user activates an entry in "open threats without a control"
- **THEN** the Threats tab opens with that threat selected

#### Scenario: Empty model
- **WHEN** the threat model has no components
- **THEN** the Overview shows zero counts and a prompt to register the first component, not an error

### Requirement: Tenant and role enforcement
Every threat model read and write SHALL be scoped to the caller's tenant, and every mutating request SHALL require the Editor role, as for all other resources.

#### Scenario: Other tenant
- **WHEN** a caller requests the threat model of a project belonging to another tenant
- **THEN** the response is "not found" and no data is returned

#### Scenario: Read-only user
- **WHEN** a caller with only the User role tries to create a threat
- **THEN** the request is rejected as forbidden
