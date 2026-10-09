# Design

## Context

`CanvasComponent` holds zoom (0.3 to 3), `panX`/`panY` in canvas units (pan is applied before zoom), `centerView` (centres without changing zoom), `revealElement` (pans an element into view; used by validation) and one `document:keydown` handler that ignores events from inputs. States have `tabindex="0"` and select on Enter. Groups are rectangles that do not own member ids: membership is derived from geometry (`groupMembers`). The store owns undo. `canvas.scss` is near its budget. See proposal.md.

## Goals / Non-Goals

**Goals:**
- Each feature is a small, separately testable unit; the canvas only wires them.

**Non-Goals:**
- Nested groups and sub-models (a separate, larger item in `feature.md`).
- Saving per-user view state (zoom, pan) on the server.

## Decisions

- **Viewport maths in a pure module `state/viewport.ts`:** `fitTo(box, view, margin, zoomRange) -> {zoom, panX, panY}` is used by zoom-to-fit, zoom-to-selection and `revealElement`. The minimap's mapping between minimap and canvas coordinates lives there too.
- **Search** is computed from the store (`searchElements(query)`): case-insensitive substring matching over state labels (line breaks as spaces), transition events and guards, ranked by states first and then by position. The search box is a small overlay in the toolbar. Ctrl+F is captured only while the canvas or the overlay has focus, so the browser's own find still works elsewhere on the page.
- **Minimap** is its own component: an SVG with state rectangles only (no labels) and the viewport rectangle, bottom-right of the canvas, 180x120 px. Its visibility is in local storage (`editor-minimap`), wrapped like the existing layout key.
- **Collapse** adds `collapsed?: boolean` to `CanvasGroup` (saved as `layout.groups[].collapsed`). Membership is snapshotted when collapsing, so members are hidden by id, not by geometry. A transition with exactly one end inside is drawn to the box's nearest side. Transitions with both ends inside are hidden. Generation, validation and coverage are unaffected. Validation's "reveal" expands first.
- **Snapping** happens in the drag handler: grid first (8 px), then guides within 6 screen pixels against the other states' left, centre and right (and top, middle and bottom), choosing the closest. Guide lines are a transient signal. Snapping is a toggle in the toolbar (remembered per browser), and Alt suspends it.
- **Keyboard:** reading order is sorted by `y`, then `x`, of state centres, in rows 40 px tall. Arrow moves use `moveNode` plus a checkpoint per key burst (debounced 500 ms), so holding a key is one undo step. Shortcuts live in one table that drives both the handler and the `?` help, so they cannot drift apart.

## Risks / Trade-offs

- [Ctrl+F conflicts with browser find] → Only while the canvas has focus. The help says so, and Escape returns focus.
- [Collapsed box and geometric group membership disagree after a drag] → Members are snapshotted at collapse, and moving a collapsed box moves its members by the same delta.
- [Keyboard moves flood autosave] → Saves are already debounced 2 s.
- [Style budget] → The minimap and search overlay are separate components with their own styles, and guide styles are a few lines.
