# Proposal

## Why

Real models are usually larger than 30 states, but the canvas only offers zoom buttons, a pan tool and "centre" (which centres without fitting). Finding a state means scanning the diagram, there is no overview of where the view is, groups cannot be folded away, states are placed freehand without snapping, and keyboard use stops at selecting with Enter and deleting. That is slow for everyone and short of NFR-004 (keyboard alternative) for keyboard users.

## What Changes

- **Search and jump:** Ctrl+F (or a toolbar search box) finds states and transitions by name, event or guard. Picking a hit selects it and brings it into view, and next/previous steps through the hits.
- **Zoom to fit and zoom to selection:** fit the whole model, or the selection, in view (toolbar and shortcuts `Shift+1` / `Shift+2`).
- **Minimap:** an overview in a canvas corner shows the whole model and the current viewport. Dragging or clicking it pans the view. It can be hidden, and the choice is remembered per browser.
- **Collapse groups:** a group can be collapsed into a single box labelled with its name and member count. Transitions to and from members attach to the box. Expanding restores the members where they were. The collapsed state is saved with the layout.
- **Snap and guides:** while dragging, states snap to an 8 px grid (toggle, `Alt` suspends it) and to the edges and centres of nearby states, which are shown as guide lines.
- **Keyboard navigation:** Tab and Shift+Tab move between states, Ctrl+Tab moves to transitions, arrow keys move the selection (Shift for 10 steps), F2 renames, `+` and `-` zoom, and a visible focus ring follows. All are listed in a shortcuts help (`?`).
- No API change. Collapsed groups use the layout from `persist-canvas-layout`.

## Capabilities

### New Capabilities
- `canvas-navigation`: Finding, viewing and moving through model elements on the canvas, by mouse and by keyboard.

### Modified Capabilities

## Impact

- `frontend/src/features/models/components/canvas/` (toolbar, minimap component, key handling, drag snapping), the store (group collapse, keyboard move), layout mapping (`groups[].collapsed`), the shortcuts help.
- `docs/specification/06-ui.md` (Canvas tools, a keyboard table).
- Partly meets NFR-004. The table view remains in `model-review-modes`.
