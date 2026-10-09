# Spec Delta

## ADDED Requirements

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
