# Tasks

## 1. Geometry

- [x] 1.1 Create `state/edge-geometry.ts` with straight, curved (`curve`), waypoint spline, right-angle and self-loop-by-side paths plus label and handle points; verify unit tests per style, anchor sides, multiple self-loops on one side, parallel fan-out, and that the bend-handle point lies on the drawn curve
- [x] 1.2 Extend `CanvasEdge` with `waypoints`, `routing` and `labelOffset`, with store operations (set curve, add/move/remove waypoint, set routing, set/reset label offset, straighten) that checkpoint undo, and move inner waypoints when a group moves; verify store tests including undo/redo

## 2. Canvas

- [x] 2.1 Move transition rendering and handles into `components/canvas-edges/` using the geometry module, with no visual change for existing edges; verify the existing canvas tests pass unchanged and `canvas.scss` shrinks
- [x] 2.2 Bend handle: drag to bend, double-click to straighten; verify a component test that dragging sets the curve and double-click resets it
- [x] 2.3 Waypoints: Alt+click and context-menu "Add bend point" insert in path order, drag moves, double-click removes; verify component tests
- [x] 2.4 Label dragging with leader line and "Reset label position" in the context menu; verify a component test for drag, leader line and reset
- [x] 2.5 Self-loops drawn by anchor side and spread when several share a side; verify a rendering test for two loops on one side

## 3. Panel, persistence, docs

- [x] 3.1 Properties panel: routing style select, Straighten, waypoint list with remove buttons, Reset label position; verify a component test for each control
- [x] 3.2 Store the new edge keys in layout v1 (`layout.transitions[<id>]`), only when set; verify mapping round-trip tests (requires `persist-canvas-layout`)
- [x] 3.3 Document transition routing in `06-ui.md` (including that right-angle routing does not avoid states); run `ng lint`, `ng test`, `ng build` (no style budget error) and verify all pass
