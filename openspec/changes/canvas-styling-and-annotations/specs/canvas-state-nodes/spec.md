# Spec Delta

## ADDED Requirements

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
