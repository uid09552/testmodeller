# model-editor-persistence Specification

## Purpose
Defines what the model editor saves and restores, so that a reload shows the model as it was left and saving never loses data the editor does not edit.

## Requirements

### Requirement: Layout survives a reload
The editor SHALL store its presentation of a model with the model and restore it when the model is opened: each state's shape, colour, size and decision kind, the groups, each transition's curve and connector points, and the display numbers of test cases.

#### Scenario: Reload after layout work
- **WHEN** a user changes a state's shape and colour, groups two states and bends a transition, then reloads the page
- **THEN** the model is shown with the same shapes, colours, group and bend

#### Scenario: Another browser
- **WHEN** a second user opens the same model in another browser
- **THEN** they see the same layout

#### Scenario: Layout-only edit is saved
- **WHEN** the only change is presentation, such as a colour
- **THEN** it is saved like any other change and survives a reload

### Requirement: Missing or partial layout falls back to defaults
A model without stored layout, or whose layout does not mention an element, SHALL be shown with the default presentation for that element, and layout entries for elements that no longer exist SHALL be ignored and dropped on the next save.

#### Scenario: Model created through the API
- **WHEN** a model without layout is opened
- **THEN** every state has its kind's default shape and colour and every transition is straight

#### Scenario: Element deleted elsewhere
- **WHEN** the stored layout mentions a state that was deleted
- **THEN** the model opens normally and the next save no longer stores that entry

### Requirement: Layout has no effect on behaviour
Layout SHALL NOT change validation, generation, coverage, staleness or any other result computed from a model, and the backend SHALL store and return it without interpreting it.

#### Scenario: Generate with and without layout
- **WHEN** the same graph is generated once with layout and once without
- **THEN** the generated test cases are identical

### Requirement: Layout is bounded and must be an object
The API SHALL accept layout only as a JSON object of at most 256 KB and SHALL reject anything else with a validation error naming the field, without changing the model.

#### Scenario: Oversized layout
- **WHEN** a save sends a layout larger than 256 KB
- **THEN** it is rejected with a validation error on `layout` and the stored model is unchanged

#### Scenario: Not an object
- **WHEN** a save sends a layout that is an array or a string
- **THEN** it is rejected with a validation error on `layout`

### Requirement: Layout travels with the model
Layout SHALL be part of a model's version snapshots, of a duplicated model, and of the JSON export and import.

#### Scenario: Restore a version
- **WHEN** a user restores an earlier version of a model
- **THEN** the layout of that version is restored with its graph

#### Scenario: Duplicate
- **WHEN** a model is duplicated
- **THEN** the copy has the same layout, with its element references pointing at the copy's elements

#### Scenario: Export and import
- **WHEN** a project is exported as JSON and imported again
- **THEN** the imported models have their layout

### Requirement: Saving keeps what the editor does not edit
A save from the editor SHALL keep the model's variables, and any other model content the editor does not edit, exactly as stored.

#### Scenario: Model with variables
- **WHEN** a model that has variables is opened in the editor, a state is moved, and the change is saved
- **THEN** the model still has the same variables with the same types and initial values
