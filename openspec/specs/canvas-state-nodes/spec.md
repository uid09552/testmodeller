# canvas-state-nodes Specification

## Purpose
Defines how states are sized and labelled on the model canvas so that their names stay readable and never overlap other diagram elements.

## Requirements

### Requirement: State size fits its label
The system SHALL size each state automatically so its full label is inside the state's outline, without user action.

#### Scenario: Long name on creation or rename
- **WHEN** a state's name is longer than fits its current size
- **THEN** the state grows (wrapping the label onto further lines beyond a maximum width) until the label fits

#### Scenario: Live while editing
- **WHEN** the user types a longer name in the inline editor
- **THEN** the state resizes as the text changes

#### Scenario: Shortening a name
- **WHEN** the name is shortened
- **THEN** the state shrinks to fit, but never below its shape's default size

#### Scenario: Loaded model
- **WHEN** a model with an overflowing name is opened
- **THEN** the state is displayed at a size that fits the label

### Requirement: Resizing keeps the diagram connected
Changing a state's size SHALL keep its position centre fixed, and its transitions, connector points and test chips SHALL follow the new outline.

#### Scenario: Transition after resize
- **WHEN** a state with attached transitions grows
- **THEN** each transition ends on the new outline of the state
