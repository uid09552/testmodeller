# coverage-gaps Specification

## Purpose
Lets users see at a glance which states and transitions of a model are not covered by any test case, directly on the editor canvas.

## Requirements

### Requirement: Coverage overlay on the canvas
The editor SHALL offer a toggle that highlights uncovered states and transitions and shows covered and total counts, without changing the model.

#### Scenario: Turn the overlay on
- **WHEN** the user switches coverage on
- **THEN** states without any test case and transitions without an assigned test case are highlighted, and the toolbar shows e.g. "3/5 states, 4/7 transitions"

#### Scenario: Turn it off
- **WHEN** the user switches coverage off
- **THEN** the canvas returns to its normal appearance

#### Scenario: Not colour alone
- **WHEN** an element is highlighted as uncovered
- **THEN** it also carries a non-colour marker and an accessible label saying it is uncovered

### Requirement: Choice of coverage dimension
The user SHALL be able to show uncovered states, uncovered transitions, or both.

#### Scenario: Transitions only
- **WHEN** the user chooses transitions
- **THEN** only uncovered transitions are highlighted and counted

### Requirement: Live state coverage
Highlighting of uncovered states SHALL follow the tests in the editor immediately, without a save.

#### Scenario: Add a test to an uncovered state
- **WHEN** the user adds a test case to a highlighted state
- **THEN** the state stops being highlighted and the counts update at once

### Requirement: Transition coverage reflects the saved model
Transition coverage SHALL come from the server's coverage of the saved model and SHALL be labelled as of the last save while the model has unsaved changes.

#### Scenario: Unsaved model
- **WHEN** the model has unsaved changes and transition coverage is shown
- **THEN** the counts are marked as "as of last save"

#### Scenario: Never saved
- **WHEN** the model has not been saved yet
- **THEN** transition coverage is shown as unavailable rather than as zero

### Requirement: Jump from a gap to a fix
Selecting an uncovered state SHALL offer a way to add a test case to it.

#### Scenario: Add from the gap
- **WHEN** the user selects a highlighted state
- **THEN** the Test Cases tab opens for it ready to add a case
