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

### Requirement: State names support line breaks
A state's name SHALL be able to contain explicit line breaks, which are displayed as line breaks on the canvas.

#### Scenario: Insert a break while editing on the canvas
- **WHEN** the user presses Shift+Enter in the inline name editor
- **THEN** a line break is inserted and editing continues

#### Scenario: Commit and cancel keys
- **WHEN** the user presses Enter in the inline editor
- **THEN** the name is committed with its line breaks; Escape discards the edit

#### Scenario: Edit in the Properties panel
- **WHEN** the user enters a line break in the Properties panel label field
- **THEN** the state's name contains the break and the canvas shows it

#### Scenario: Name shown elsewhere
- **WHEN** a name with line breaks appears outside the canvas (validation message, explorer tree, test reference)
- **THEN** each line break is shown as a space

### Requirement: Sizing accounts for explicit line breaks
Automatic sizing SHALL treat each explicit line separately, wrapping an over-long line as before, and SHALL size the state to fit all resulting lines.

#### Scenario: Short lines stacked
- **WHEN** a name has three short lines
- **THEN** the state is at least tall enough for three lines and wide enough for the widest

#### Scenario: Break plus long line
- **WHEN** one explicit line is longer than the maximum text width
- **THEN** it wraps further and the state grows to fit

#### Scenario: Blank lines
- **WHEN** a name has blank lines or trailing breaks
- **THEN** blank lines are kept in the middle and trailing blank lines are dropped on commit

#### Scenario: Live while typing
- **WHEN** the user adds or removes a line break in the inline editor
- **THEN** the state resizes immediately, and the editor field fits the state

### Requirement: State shapes
A state SHALL be drawable as a circle, rectangle, diamond, hexagon, parallelogram, cylinder or document, chosen in the Properties panel and the context menu. The shape SHALL NOT change the state's kind, and every shape SHALL offer the same four connector points.

#### Scenario: Make a state a cylinder
- **WHEN** the user chooses "Cylinder" for a regular state
- **THEN** the state is drawn as a cylinder, its kind is still regular, and its label fits inside the cylinder's body

#### Scenario: Transitions on a slanted shape
- **WHEN** a parallelogram state has transitions on its left and right connector points
- **THEN** each transition ends on the parallelogram's outline

#### Scenario: Older editor data
- **WHEN** a model whose layout names a shape this editor does not know is opened
- **THEN** that state is drawn with its kind's default shape

### Requirement: Sizing accounts for the label's text style
Automatic state sizing SHALL measure the label with its font size, weight and style, so that a larger or bold label fits inside the state.

#### Scenario: Larger font
- **WHEN** the user changes a state's label from normal to large
- **THEN** the state grows around its centre so the label still fits

#### Scenario: Smaller font
- **WHEN** the user changes the label back to normal
- **THEN** the state shrinks to fit, but never below its shape's default size
