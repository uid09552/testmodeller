# Spec Delta

## Purpose

Lets users place notes and free text on the model canvas to explain a diagram, without those elements becoming part of the model or affecting anything computed from it.

## ADDED Requirements

### Requirement: Add an annotation
The editor SHALL offer two annotation kinds, a note (a filled box with a folded corner) and a text box (text without a border or fill), which the user can drag from the palette onto the canvas.

#### Scenario: Drop a note
- **WHEN** the user drags the note tool onto empty canvas
- **THEN** a note is placed there with its text editor open

#### Scenario: Drop a text box
- **WHEN** the user drags the text tool onto the canvas
- **THEN** a text box is placed there with its text editor open

### Requirement: Edit annotation text
An annotation's text SHALL be editable in place by double-click or F2 and in the Properties panel. It SHALL support several lines and at most 2,000 characters. An empty annotation SHALL be removed when its editor closes.

#### Scenario: Multi-line note
- **WHEN** the user types two lines separated by Shift+Enter and presses Enter
- **THEN** the note shows both lines

#### Scenario: Long text
- **WHEN** the user pastes 3,000 characters into a note
- **THEN** the text is cut at 2,000 characters and the editor says so

#### Scenario: Empty annotation
- **WHEN** the user closes the editor of a new note without typing
- **THEN** the note is removed

### Requirement: Move, resize and delete annotations
Annotations SHALL be selectable, movable, resizable by their handles, deletable, and part of marquee selection, alignment and snapping like states. Text SHALL wrap to the annotation's width, and a note SHALL grow in height to fit its text.

#### Scenario: Resize a note
- **WHEN** the user drags a note's right handle to make it narrower
- **THEN** its text wraps to the new width and the note grows taller to fit

#### Scenario: Move with states
- **WHEN** a marquee selects two states and a note and the user drags them
- **THEN** all three move together, as one undo step

### Requirement: Style annotations
Annotations SHALL support the text style controls, and notes SHALL also support fill colour and line style. A new note SHALL use a pale yellow fill by default.

#### Scenario: Restyle a note
- **WHEN** the user sets a note's fill to pale blue and its border to dashed
- **THEN** the note is drawn pale blue with a dashed border

### Requirement: Annotations are not part of the model
Annotations SHALL NOT appear in or affect validation, test generation, coverage, staleness, simulation, search results for states and transitions, the table view, AI context or proposals, or exported test cases. Transitions SHALL NOT start or end on an annotation.

#### Scenario: Generate with notes
- **WHEN** the same model is generated once with notes on its canvas and once without
- **THEN** the generated test cases are identical

#### Scenario: Connect to a note
- **WHEN** the user drags a transition's end onto a note
- **THEN** no transition is created to the note

#### Scenario: Validation
- **WHEN** a model has a note that sits over no state and is linked to nothing
- **THEN** validation reports nothing about it

### Requirement: Annotations in present mode
In present mode, annotations SHALL be shown and SHALL NOT be editable, movable or deletable.

#### Scenario: Present a model with notes
- **WHEN** a model with notes is shown in present mode
- **THEN** the notes are visible and double-clicking one does not open its editor
