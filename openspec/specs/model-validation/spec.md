# model-validation Specification

## Purpose
Defines how validation results are surfaced in the model editor and how users navigate from a result to the element it concerns.

## Requirements

### Requirement: Status badge opens the validation list
The editor toolbar's error or warning count SHALL be an interactive control that opens the Validation tab of the bottom panel.

#### Scenario: Click the error badge
- **WHEN** the model has 1 error and the user clicks the "1 error" badge
- **THEN** the Validation tab is shown with the issue list, and the bottom panel is visible

#### Scenario: Warnings only
- **WHEN** the model has only warnings and the user clicks the warning badge
- **THEN** the Validation tab is shown

#### Scenario: Valid model
- **WHEN** the model has no issues
- **THEN** the toolbar shows the valid indicator, which does not open the tab

#### Scenario: Keyboard
- **WHEN** the badge has focus and the user presses Enter or Space
- **THEN** the Validation tab is shown

### Requirement: Issue rows navigate to their element
Activating an issue row SHALL select the state or transition it concerns and bring it into view on the canvas.

#### Scenario: Issue on a state
- **WHEN** the user clicks an issue that concerns a state
- **THEN** that state is selected and visible in the canvas viewport

#### Scenario: Issue on a transition
- **WHEN** the user clicks an issue that concerns a transition
- **THEN** that transition is selected, not a state, and visible in the canvas viewport

#### Scenario: Issue without an element
- **WHEN** an issue has no element
- **THEN** the row is not a navigation control
