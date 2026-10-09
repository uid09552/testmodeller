# canvas-navigation Specification

## Purpose
Lets users find, view and move through the elements of large models on the canvas, with the mouse and with the keyboard alone.

## Requirements

### Requirement: Search and jump
The editor SHALL let the user search the model's states and transitions by name, event or guard, and SHALL select and bring into view the hit the user picks.

#### Scenario: Find a state
- **WHEN** the user presses Ctrl+F, types "pay" and picks the hit "Payment"
- **THEN** the state "Payment" is selected and visible in the canvas

#### Scenario: Step through hits
- **WHEN** a search has several hits and the user presses Enter or Shift+Enter
- **THEN** the next or previous hit is selected and brought into view, and the position is shown, e.g. "2 of 5"

#### Scenario: No hits
- **WHEN** nothing matches
- **THEN** the search says so and the selection is unchanged

### Requirement: Zoom to fit and to selection
The editor SHALL zoom and pan so that the whole model, or the current selection, fills the view with a margin, within the allowed zoom range.

#### Scenario: Fit the model
- **WHEN** the user chooses "Zoom to fit" or presses Shift+1
- **THEN** every state is visible and the model fills the view

#### Scenario: Fit the selection
- **WHEN** two distant states are selected and the user presses Shift+2
- **THEN** both are visible and fill the view

### Requirement: Minimap
The canvas SHALL show an overview of the whole model with the current viewport marked. Clicking or dragging in it SHALL pan the canvas. The user SHALL be able to hide it.

#### Scenario: Pan with the minimap
- **WHEN** the user drags the viewport rectangle in the minimap
- **THEN** the canvas pans to match

#### Scenario: Hidden
- **WHEN** the user hides the minimap and reloads
- **THEN** it stays hidden in that browser

### Requirement: Collapse groups
A group SHALL be collapsible into a single box showing its name and number of states. Transitions to and from its states SHALL attach to the box while it is collapsed. Expanding SHALL restore its states unchanged.

#### Scenario: Collapse
- **WHEN** the user collapses a group of four states
- **THEN** a box labelled with the group name and "4 states" replaces them, and transitions from outside end on the box

#### Scenario: Expand
- **WHEN** the user expands it again
- **THEN** the four states and their transitions appear where they were

#### Scenario: Saved
- **WHEN** a model with a collapsed group is reloaded
- **THEN** the group is still collapsed

#### Scenario: Selected element inside
- **WHEN** search, validation or a test reveals a state inside a collapsed group
- **THEN** the group expands and the state is selected

### Requirement: Snap and alignment guides
While a state is dragged, it SHALL snap to a grid and to the edges and centres of nearby states, and the guides it snaps to SHALL be shown. Snapping SHALL be switchable off and SHALL be suspended while Alt is held.

#### Scenario: Align with a neighbour
- **WHEN** the user drags a state near the horizontal centre line of another state
- **THEN** it snaps to that line and a guide line is shown until the drag ends

#### Scenario: Free placement
- **WHEN** the user holds Alt while dragging
- **THEN** the state follows the pointer without snapping

### Requirement: Keyboard navigation
All canvas navigation and the basic editing of positions SHALL be possible with the keyboard alone, with a visible focus indicator.

#### Scenario: Move between states
- **WHEN** the canvas has focus and the user presses Tab
- **THEN** focus and selection move to the next state in reading order (top to bottom, left to right), with a visible focus ring, and Shift+Tab moves back

#### Scenario: Move a state
- **WHEN** a state is selected and the user presses an arrow key, or Shift with an arrow key
- **THEN** the state moves by one grid step, or by ten, and the move is undoable

#### Scenario: Rename and zoom
- **WHEN** the user presses F2 on a selected state, or `+` / `-`
- **THEN** the inline name editor opens, or the canvas zooms in or out

#### Scenario: Shortcut help
- **WHEN** the user presses `?`
- **THEN** a list of all canvas shortcuts is shown
