# Spec Delta

## Purpose

Lets users set the line, fill and text style of states and transitions on the model canvas, so that a diagram can show emphasis and grouping beyond its structure, while the model's behaviour stays unchanged.

## ADDED Requirements

### Requirement: Line style
States and transitions SHALL each have a line pattern (solid, dashed or dotted), a line width from 1 to 4 px, and a line colour, set in the Properties panel. Without a user choice, each element keeps its current default appearance.

#### Scenario: Dash a transition
- **WHEN** the user selects a transition and sets its line pattern to dashed and its width to 3 px
- **THEN** the transition is drawn as a 3 px dashed line, including its curve, waypoints or right-angle segments

#### Scenario: Dotted state border
- **WHEN** the user sets a state's line pattern to dotted and its line colour to red
- **THEN** the state's outline is drawn dotted in red, whatever its shape

#### Scenario: Back to default
- **WHEN** the user chooses "Default" for the line colour of a styled state
- **THEN** the state's outline returns to its kind's default colour

### Requirement: Arrowhead style
A transition SHALL have an arrowhead style of filled (default), open or line ("V"), drawn in the transition's line colour. A transition SHALL always show an arrowhead at its target.

#### Scenario: Open arrowhead
- **WHEN** the user sets a transition's arrowhead to open
- **THEN** the transition ends in an unfilled triangle at its target state, in the transition's line colour

#### Scenario: No option without arrowhead
- **WHEN** the user opens the arrowhead control
- **THEN** it offers no choice that removes the arrowhead

### Requirement: Fill colour
A state SHALL have a fill colour. When the fill is dark, the state's label SHALL switch to a light text colour unless the user set a text colour.

#### Scenario: Dark fill
- **WHEN** the user sets a state's fill to dark slate and has not set a text colour
- **THEN** the state is filled dark slate and its label is drawn in a light colour

### Requirement: Text style
States, transition labels and annotations SHALL each have a text colour, a font size of small, normal or large, bold and italic. A transition's guard text SHALL follow its label's colour and size.

#### Scenario: Bold large state label
- **WHEN** the user sets a state's label to large and bold
- **THEN** the label is drawn large and bold, and the state grows so the label fits

#### Scenario: Coloured transition label
- **WHEN** the user sets a transition's text colour to green and italic
- **THEN** its label and guard are drawn green and italic, and the label's background still makes them readable

### Requirement: Colour choice
Every colour control SHALL offer a "Default" choice, the preset palette, and a custom colour picker. A custom colour SHALL be shown as the selected value afterwards.

#### Scenario: Custom colour
- **WHEN** the user picks #123456 with the custom colour picker for a state's fill
- **THEN** the state is filled #123456 and the control shows #123456 as the current value

### Requirement: Style a selection
When several states, transitions or annotations are selected, the Properties panel SHALL show the style controls that apply to all of them. Changing a control SHALL apply it to every selected element that supports it, as one undoable step.

#### Scenario: Dash several transitions
- **WHEN** three transitions are selected and the user sets the line pattern to dashed
- **THEN** all three become dashed, and one Ctrl+Z restores all three

#### Scenario: Mixed selection
- **WHEN** a state and a transition are selected
- **THEN** the panel offers line pattern, width, line colour and text style, but not fill or arrowhead

#### Scenario: Differing values
- **WHEN** the selected elements have different line widths
- **THEN** the width control shows a mixed state, and choosing a value sets it on all of them

### Requirement: Status cues take precedence over user style
While coverage, path highlight, simulation or selection marks an element, the canvas SHALL draw that cue over the element's user style, and the cue SHALL stay distinguishable from any user style. The user style SHALL return when the cue ends.

#### Scenario: Uncovered dashed transition
- **WHEN** a transition the user styled as dashed and blue is uncovered under the coverage overlay
- **THEN** it is drawn with the coverage cue, including its non-colour marker, and returns to dashed blue when the overlay is turned off

#### Scenario: Selected styled state
- **WHEN** a state with a dotted red border is selected
- **THEN** the selection outline is visible around it

### Requirement: Styling is presentation only
Style changes SHALL be undoable, SHALL be shown in present mode, and SHALL NOT change the model's states, transitions or any result computed from them.

#### Scenario: Undo a style change
- **WHEN** the user sets a state's fill and presses Ctrl+Z
- **THEN** the state's previous fill is restored

#### Scenario: Present mode
- **WHEN** a styled model is shown in present mode
- **THEN** every element is drawn with its style, and no style control is offered
