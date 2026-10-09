# Spec Delta

## Purpose

Gives keyboard and screen-reader users, and anyone who prefers a list, a complete alternative to the canvas for viewing and editing a model's states and transitions (NFR-004).

## ADDED Requirements

### Requirement: Table view of a model
The editor SHALL offer a table view showing the model's states (name, kind, number of test cases, number of outgoing transitions) and its transitions (source, event, guard, action, expected result, target) as tables with column headers.

#### Scenario: Switch to the table view
- **WHEN** the user switches from canvas to table view
- **THEN** every state and every transition of the model is listed, and switching back shows the canvas unchanged

### Requirement: Edit in the table view
Every field a user can edit on the canvas or in the Properties panel for a state or transition SHALL be editable in the table view. Rows SHALL be addable and deletable, and edits SHALL share the canvas's undo history and be saved the same way.

#### Scenario: Change a guard
- **WHEN** the user edits a transition's guard in the table and switches to the canvas
- **THEN** the canvas shows the new guard, and Ctrl+Z restores the old one

#### Scenario: Add a transition
- **WHEN** the user adds a transition row, choosing its source and target states from lists
- **THEN** the transition exists in the model and appears on the canvas

#### Scenario: Delete a state
- **WHEN** the user deletes a state row
- **THEN** the state and its transitions are removed, after the same confirmation rules as on the canvas

### Requirement: Fully keyboard operable
The table view SHALL be operable with the keyboard alone: moving between cells, starting and committing an edit, adding and deleting rows, and switching views, with a visible focus indicator and accessible names for every control.

#### Scenario: Edit without a mouse
- **WHEN** a keyboard user tabs into the transitions table, moves to a guard cell with the arrow keys, presses Enter, types, and presses Enter again
- **THEN** the guard is changed and focus stays on that cell

#### Scenario: Validation in the table
- **WHEN** a state has a validation issue
- **THEN** its row shows the issue as text and the issue is part of the row's accessible description
