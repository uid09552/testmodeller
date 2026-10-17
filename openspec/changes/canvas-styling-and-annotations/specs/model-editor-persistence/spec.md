# Spec Delta

## MODIFIED Requirements

### Requirement: Layout survives a reload
The editor SHALL store its presentation of a model with the model and restore it when the model is opened: each state's shape, colour, size, decision kind and style, the groups, each transition's curve, connector points and style, the annotations, and the display numbers of test cases.

#### Scenario: Reload after layout work
- **WHEN** a user changes a state's shape and colour, groups two states and bends a transition, then reloads the page
- **THEN** the model is shown with the same shapes, colours, group and bend

#### Scenario: Another browser
- **WHEN** a second user opens the same model in another browser
- **THEN** they see the same layout

#### Scenario: Layout-only edit is saved
- **WHEN** the only change is presentation, such as a colour
- **THEN** it is saved like any other change and survives a reload

#### Scenario: Reload after styling and notes
- **WHEN** a user dashes a transition, fills a state, makes its label bold, adds a note, and reloads the page
- **THEN** the transition is dashed, the state filled with a bold label, and the note is where it was with its text and style

#### Scenario: Duplicate with notes
- **WHEN** a model with notes is duplicated
- **THEN** the copy shows the same notes, and editing a note in the copy does not change the original
