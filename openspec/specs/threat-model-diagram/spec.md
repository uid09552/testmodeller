# threat-model-diagram Specification

## Purpose
Shows a threat model's architecture as an editable data-flow diagram — components as shapes, flows as arrows, zones as regions — so the user can build and read the architecture visually and see where flows cross trust boundaries.

## Requirements

### Requirement: Diagram shows the architecture
The Diagram tab SHALL draw every architecture component as a shape chosen by its type's layer (external entities as rectangles, data components as cylinders, the rest as rounded rectangles) with its name and type icon, every data flow as an arrow from source to target with its label, and every zone as a labelled region containing its components.

#### Scenario: Database shape
- **WHEN** the threat model contains a component of type relational database
- **THEN** the diagram draws it as a cylinder labelled with its name

#### Scenario: Zone region
- **WHEN** two components are in zone "Internal"
- **THEN** both are drawn inside a region labelled "Internal" with its trust level

### Requirement: Edit the architecture on the diagram
The diagram SHALL let the user add a component from a palette of types, add a flow by dragging from one component's connector dot to another, open an entity's editor by double-click or Enter, move a component into or out of a zone region by dragging (which changes its zone), and delete the selection. Every such edit SHALL be saved as the same entity change the tables make.

#### Scenario: Drag a flow
- **WHEN** the user drags from "Web app" to "Customer API"
- **THEN** a "calls" flow is created between them and its editor opens

#### Scenario: Move into a zone
- **WHEN** the user drags "Customer API" into the "DMZ" region
- **THEN** the component's zone becomes "DMZ" and its flows' crossings are recomputed

#### Scenario: Same data as tables
- **WHEN** the user adds a component on the diagram and then opens the Components tab
- **THEN** the component is listed there

### Requirement: Trust-boundary crossings are visible
A flow that crosses a trust boundary SHALL be drawn with a distinct marker on the line at the region edge and an accessible name that ends "crosses trust boundary", so the cue does not rely on colour.

#### Scenario: Crossing marker
- **WHEN** a flow goes from the "Internet" region to the "Internal" region
- **THEN** a boundary marker is drawn where the arrow leaves the "Internet" region and the arrow's accessible name ends "crosses trust boundary"

### Requirement: Threat overlay
A "Threats" toggle on the diagram toolbar SHALL show, on each component and flow, a badge with the number of its open threats and the highest risk level among them, as a symbol and a word.

#### Scenario: Badge
- **WHEN** the overlay is on and "Customer API" has two open threats, one High and one Moderate
- **THEN** its badge shows "2" and "High"

#### Scenario: Open threats from the badge
- **WHEN** the user activates a component's threat badge
- **THEN** the Threats tab opens filtered to that component

### Requirement: Diagram navigation
The diagram SHALL support zoom in/out, zoom to fit, pan, snapping while dragging, a minimap and keyboard selection of components (Tab / Shift+Tab), with the same keys as the model editor where the action exists there.

#### Scenario: Zoom to fit
- **WHEN** the user presses Shift+1 with the diagram focused
- **THEN** the whole diagram is brought into view

### Requirement: Diagram layout is persisted
Positions, sizes, zone region bounds and flow routing SHALL be saved with the threat model as a presentation-only layout. A reload SHALL show the diagram as it was left; an entity the layout does not mention SHALL be placed automatically; layout entries for deleted entities SHALL be dropped on the next save. The layout SHALL have no effect on crossings, threats, risks or tests.

#### Scenario: Reload
- **WHEN** the user moves components, reloads the page and opens the Diagram tab
- **THEN** the components are where they were left

#### Scenario: Component added through the API
- **WHEN** a component is created through the API without a layout entry
- **THEN** the diagram places it without overlapping existing components

### Requirement: Flows attach to connector dots
Every component SHALL offer four connector dots (top, right, bottom, left), as states do in the model editor. A flow drawn on the diagram SHALL start at the dot it was dragged from and end at the target's dot nearest the drop. A flow without stored dots (created in a table or through the API) SHALL attach to the facing sides of its components, never centre to centre.

#### Scenario: Drawn flow keeps its dots
- **WHEN** the user drags from the bottom dot of "Customer API" and drops near the top of "Orders DB"
- **THEN** the arrow runs from the bottom dot of "Customer API" to the top dot of "Orders DB"

#### Scenario: Flow from a table
- **WHEN** a flow between two side-by-side components was created on the Data Flows tab
- **THEN** it joins the right dot of the left component to the left dot of the right one


### Requirement: Re-attach flow ends
Dragging a selected flow's end onto another dot SHALL re-attach it there; dropping it on another component SHALL change the flow's source or target, saved like a table edit.

#### Scenario: Re-attach to another component
- **WHEN** the user drags the target end of a flow onto another component
- **THEN** the flow's target becomes that component and the change is saved like a table edit

### Requirement: Flow routing
A flow SHALL be curved (default) or right-angle, as in the model editor. A selected curved flow SHALL show a bend handle to drag; "Straighten" SHALL remove the bend. Routing is presentation only and saved in the layout.

#### Scenario: Right-angle routing
- **WHEN** the user chooses "Right-Angle Routing" on a flow
- **THEN** the flow is drawn with right-angle segments and stays so after a reload

### Requirement: Element styles
Components, flows and zones SHALL be styleable as in the model editor, on the Properties tab of the detail view: line colour, pattern and width; fill (components and zones); text colour, size, bold and italic; the arrowhead of flows; and the shape of components. Styles are presentation only and saved in the layout; selection, crossing markers and threat badges stay distinguishable by more than colour.

#### Scenario: Properties tab
- **WHEN** the user chooses "Properties…" in a component's right-click menu
- **THEN** its detail view opens on the Properties tab

#### Scenario: Dashed red flow
- **WHEN** the user opens a flow's properties and sets its line to dashed and red
- **THEN** the flow and its arrowhead are drawn dashed and red, and stay so after a reload


### Requirement: Style several elements at once
Shift+click SHALL add an element to the selection; a style change SHALL apply to every selected element that supports it, and a control whose values differ SHALL show as mixed.

#### Scenario: Style several components
- **WHEN** the user Shift+clicks two components and sets a fill colour
- **THEN** both components get that fill

### Requirement: Detail view
Clicking a component, flow or zone SHALL open its detail view, with the tab "Details" (its name and other fields, as on the tables' editors) and the tab "Properties" (its style, shape and routing). A drag SHALL NOT open it, selecting another element SHALL replace it, and clearing the selection SHALL close it. With several elements selected, "Properties…" SHALL open a Properties panel for all of them.

#### Scenario: Click opens Details
- **WHEN** the user clicks the component "Web app" without moving it
- **THEN** its detail view opens on the Details tab beside a Properties tab

#### Scenario: Drag does not open it
- **WHEN** the user drags a component
- **THEN** it moves and no detail view opens

### Requirement: Context menus
Right-clicking a component, flow, zone or the background SHALL open a menu like the model editor's. Component, flow and zone menus SHALL offer "Edit Details…" and "Properties…", which open the detail view on that tab. Escape or a click elsewhere SHALL close the menu, and every item SHALL also be reachable without it.

#### Scenario: Reverse a flow
- **WHEN** the user chooses "Reverse Direction" on the flow "Web app → Customer API"
- **THEN** the flow becomes "Customer API → Web app" and is saved like a table edit

#### Scenario: Add here
- **WHEN** the user right-clicks the background inside the "Internal" region and chooses "Add Component Here"
- **THEN** a component of the chosen type is created at that point in zone "Internal"

### Requirement: Menu actions per element
The component menu SHALL also offer add data flow, shapes, move to a zone, show threats and delete. The flow menu SHALL offer reverse direction, curved or right-angle routing, straighten, show threats and delete. The zone menu SHALL offer fit to contents and delete. The background menu SHALL offer add a component or zone at that point and zoom to fit.

#### Scenario: Component actions
- **WHEN** the user right-clicks a component
- **THEN** the menu lists add data flow, the shapes, the zones to move to, show threats and delete
