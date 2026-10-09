# model-review Specification

## Purpose
Lets users show a model to others without risk of changing it, and see exactly which states and transitions a test case walks through.

## Requirements

### Requirement: Present mode
The editor SHALL offer a present mode that shows the model read-only: no state or transition can be added, moved, reconnected, renamed or deleted, and the editing tools, palette and side panels are hidden. Zoom, pan, search and highlights SHALL keep working.

#### Scenario: Enter present mode
- **WHEN** the user switches to present mode
- **THEN** the canvas fills the editor, the editing controls are gone, and dragging a state pans the view instead of moving it

#### Scenario: Shareable link
- **WHEN** someone opens the model's link with `?view=present`
- **THEN** the model opens in present mode

#### Scenario: Leave present mode
- **WHEN** the user presses Esc or chooses "Edit"
- **THEN** the editor returns with its panels, and nothing about the model has changed

### Requirement: Highlight a test case's path
Choosing "Show path" for a test case SHALL highlight the states and transitions it is assigned to and dim all others. Each highlighted transition and state SHALL show its step number when the assignment has one.

#### Scenario: Generated test
- **WHEN** the user chooses "Show path" for a generated test case with five steps
- **THEN** its transitions and states are highlighted with step numbers 1 to 5, a transition taken twice shows both numbers, and the rest of the model is dimmed

#### Scenario: Test assigned to one state
- **WHEN** the test case is assigned only to a state, without steps
- **THEN** that state is highlighted without a number

#### Scenario: From another page
- **WHEN** the user chooses "Show path" on the Test Cases page or in the Traceability matrix
- **THEN** the model opens with the path highlighted and the first step brought into view

#### Scenario: Clear
- **WHEN** the user presses Esc or chooses "Clear highlight"
- **THEN** the canvas returns to normal

### Requirement: Highlights do not rely on colour
Highlighted and dimmed elements SHALL differ by more than colour, and the highlight SHALL be announced to assistive technology.

#### Scenario: Screen reader
- **WHEN** a path is highlighted
- **THEN** a status message states the test case name and its number of steps, and each highlighted element's accessible name includes its step numbers
