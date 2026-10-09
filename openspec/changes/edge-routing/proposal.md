# Proposal

## Why

Transitions can be curved, but only automatically, to fan out parallel transitions. A user cannot move a transition off a state it crosses, route it around a group, place its label where it is readable, or move a self-loop off the top of a state, where it covers other loops and labels. On models with more than 30 states, which is the norm for the target teams, that makes diagrams hard to read. Once `persist-canvas-layout` stores layout, this routing work survives a reload.

## What Changes

- **Bend handle:** a selected transition shows a handle at its midpoint. Dragging it bends the transition, and double-clicking it straightens the transition again.
- **Waypoints:** the user can add bend points to a transition (Alt+click on the line, or "Add bend point" in the menu), drag them, and remove them (double-click). The transition passes through them in order.
- **Routing style per transition:** curved (default) or right-angle (orthogonal segments between the connector points and any waypoints), chosen in the Properties panel and the context menu.
- **Label placement:** the label of a transition can be dragged off the line and snaps back with "Reset label position". Guard text moves with it.
- **Self-loops:** a self-loop is drawn on the side of the state given by its connector point (top, right, bottom or left). Several self-loops on one state are spread so they do not overlap.
- **Keyboard and panel alternative:** the Properties panel offers the routing style, a straighten action, waypoint removal and label reset, so none of this needs a mouse.
- All of it is undoable and saved in the layout. No API change.

## Capabilities

### New Capabilities
- `transition-routing`: How the user shapes the path and label placement of transitions on the canvas.

### Modified Capabilities

## Impact

- `frontend/src/features/models/` (canvas edge rendering and hit-testing, store edge operations and undo, properties panel, context menu, layout mapping v1 keys under `transitions`).
- `docs/specification/06-ui.md`.
- Depends on `persist-canvas-layout` (layout storage). Without it, everything works but resets on reload.
- `canvas.scss` is at 12.6 of 14 kB, so edge rendering moves into its own component.
