# Design

## Context

`CanvasEdge` has `curve` (one perpendicular offset for a quadratic Bezier, set only by `addEdge` to fan out parallel transitions) and optional `fromAnchor`/`toAnchor`. `CanvasComponent.edgePath` draws three cases: a fixed self-loop above the state, a straight line, or a quadratic curve. `edgeLabelPt` computes the label point the same way. Transitions are drawn twice: fat hit paths below the states, and visual paths above them. Endpoint handles for reconnecting already exist on selected transitions. The store owns undo (`pushUndo` snapshots nodes, edges and groups). `canvas.ts` is about 1,100 lines and `canvas.scss` is 12.6 of 14 kB. Layout storage comes from `persist-canvas-layout` (`layout.transitions[<id>]`). See proposal.md.

## Goals / Non-Goals

**Goals:**
- Precise manual routing with cheap rendering and simple hit-testing.
- Geometry in pure functions that can be unit-tested without a DOM.

**Non-Goals:**
- Automatic obstacle-avoiding routing (finding a path around states). Right-angle routing here connects points with elbows; it does not search for a route.
- Routing in the read-only view other than displaying it (see `model-review-modes`).

## Decisions

- **Geometry module `state/edge-geometry.ts`**, pure: `edgePath(edge, from, to, siblings) -> { d, labelAt, handles }`. It covers straight, curved (one bend, from the existing `curve`), polyline through waypoints, right-angle (elbows), and self-loops by side. The canvas and tests both use it. Alternative: keep growing `canvas.ts`. Rejected: it is already the largest file, and geometry is the part that needs tests.
- **Edge data:** `CanvasEdge` gains `waypoints?: {x,y}[]`, `routing?: 'curved' | 'orthogonal'` and `labelOffset?: {dx,dy}`. The bend handle edits `curve` when there are no waypoints. With waypoints, the curved style draws a smooth Catmull-Rom spline through them, and the bend handle is replaced by the waypoint handles. Layout v1 stores the new keys only when they are set.
- **Bend handle semantics:** the dragged point is projected onto the perpendicular of the chord, and `curve` is set so the curve passes through that point. A quadratic Bezier's midpoint sits at half the control offset, so `curve = 2 * offset`. This feels like dragging the line itself.
- **Right-angle routing:** leave the source perpendicular to its anchor side for a stub of 16 px, connect through the waypoints with one elbow per segment (horizontal first when the anchor side is left or right, vertical first otherwise), and enter the target perpendicular to its side. When an end has no anchor, the nearest side towards the other end is used.
- **Self-loops by side:** the loop is drawn outward from the anchor side. The i-th loop on the same side is scaled by `1 + 0.45 * i`, and its label sits at the loop's apex.
- **Labels:** `labelOffset` is relative to the default label point, so the label follows the transition. When the offset is more than 12 px, a 1 px dashed leader line joins the label to its default point.
- **Waypoints stay absolute** when states move: the user put them there. Moving a group moves the waypoints of transitions with both ends inside the group, since dragging a group drags its content.
- **Undo:** every drag commits one checkpoint on mouse-down, like moving states.
- **Component split:** edge rendering, its handles and its styles move into `components/canvas-edges/`. It renders inside the same SVG via an `<g>` attribute selector, so `canvas.scss` stays under budget.
- **Hit-testing:** the existing fat `edge-hit` path uses the new `d`. Waypoints and the bend handle are circles with their own handlers, drawn above the visual layer like the endpoint handles.

## Risks / Trade-offs

- [Spline overshoot between close waypoints] → Use centripetal Catmull-Rom (alpha 0.5), which does not loop or cusp.
- [Right-angle routes run through states] → Accepted for now (no obstacle avoidance). Users add waypoints. Documented.
- [Old edges have only `curve`] → Still valid: no waypoints and curved style give today's drawing.
- [Many handles clutter a busy canvas] → Handles show only on the selected transition.
