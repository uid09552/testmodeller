# transition-routing Specification

## Purpose
Lets users shape the path and label placement of transitions on the model canvas, so that large diagrams stay readable, while the model's behaviour stays unchanged.

## Requirements

### Requirement: Bend a transition
A selected transition SHALL show a bend handle at its midpoint. Dragging the handle SHALL curve the transition through the handle's position, and double-clicking the handle SHALL make it straight again.

#### Scenario: Drag the bend handle
- **WHEN** the user selects a straight transition and drags its bend handle sideways
- **THEN** the transition curves through the dragged point, its label follows the curve, and both ends stay on their connector points

#### Scenario: Straighten
- **WHEN** the user double-clicks the bend handle of a curved transition
- **THEN** the transition becomes straight

### Requirement: Waypoints
A transition SHALL support any number of waypoints that it passes through in order. Users SHALL be able to add, move and remove them.

#### Scenario: Add a waypoint
- **WHEN** the user Alt+clicks on a transition's line, or chooses "Add bend point" in its context menu
- **THEN** a waypoint is added at that position, in path order, and the transition passes through it

#### Scenario: Move and remove
- **WHEN** the user drags a waypoint, then double-clicks it
- **THEN** the transition follows the dragged waypoint, and the double-click removes it

#### Scenario: States move
- **WHEN** a state at either end of a transition with waypoints is moved
- **THEN** the waypoints stay where they are and the transition's end segments follow the state

### Requirement: Routing style
Each transition SHALL have a routing style, curved or right-angle. A right-angle transition SHALL consist of horizontal and vertical segments that leave and enter its states perpendicular to the side of their connector points and pass through its waypoints.

#### Scenario: Switch to right-angle
- **WHEN** the user sets a transition's routing style to right-angle
- **THEN** it is drawn with only horizontal and vertical segments, leaving and entering the states at right angles

#### Scenario: Default
- **WHEN** a transition is created
- **THEN** its routing style is curved, and parallel transitions between the same states are still fanned out

### Requirement: Label placement
The user SHALL be able to drag a transition's label away from its default position. The label SHALL keep its offset when the transition is reshaped or its states move, and SHALL return to the default position on "Reset label position".

#### Scenario: Drag a label
- **WHEN** the user drags a transition's label
- **THEN** the label and its guard text move together, and a thin leader line connects the label to the transition when it is away from the line

#### Scenario: Reset
- **WHEN** the user chooses "Reset label position"
- **THEN** the label returns to the middle of the transition

### Requirement: Self-loop placement
A self-loop SHALL be drawn on the side of its state given by its connector point. Several self-loops on the same side SHALL be drawn with different sizes so none overlaps another.

#### Scenario: Loop on the right
- **WHEN** the user attaches a self-loop to the right connector point of a state
- **THEN** the loop is drawn to the right of the state

#### Scenario: Two loops on one side
- **WHEN** a state has two self-loops on its top side
- **THEN** both loops and their labels are visible and do not overlap

### Requirement: Routing without a mouse
Everything a user can do to a transition's shape by mouse SHALL also be possible from the Properties panel: choose the routing style, straighten, remove each waypoint, and reset the label position.

#### Scenario: Straighten from the panel
- **WHEN** a curved transition with waypoints is selected and the user activates "Straighten" in the Properties panel
- **THEN** its waypoints are removed and it is drawn straight

### Requirement: Routing is presentation only
Routing changes SHALL be undoable, SHALL be stored with the model's layout, and SHALL NOT change the model's states, transitions or any result computed from them.

#### Scenario: Undo a bend
- **WHEN** the user bends a transition and presses Ctrl+Z
- **THEN** the transition returns to its previous shape

#### Scenario: Reload
- **WHEN** a model with bent transitions, waypoints, right-angle routing and moved labels is reloaded
- **THEN** every transition is drawn as it was left
