# Tasks

## 1. Viewport and search

- [x] 1.1 Add `state/viewport.ts` (fit a box into the view within the zoom range; minimap coordinate mapping) and use it for `revealElement`; verify unit tests for fit, clamped zoom and the minimap mapping
- [x] 1.2 Zoom to fit / zoom to selection in the toolbar and on Shift+1 / Shift+2; verify component tests that all states, or the selection, are inside the view afterwards
- [x] 1.3 Search overlay (Ctrl+F while the canvas has focus, toolbar button): matching on names, events and guards, Enter / Shift+Enter stepping, "n of m", no-hit message; verify store tests for matching and component tests for stepping and reveal

## 2. Overview and structure

- [x] 2.1 Minimap component with click and drag panning and a hide toggle remembered per browser; verify component tests for panning and the remembered toggle
- [x] 2.2 Collapse and expand groups (store, rendering of the box, transitions attached to the box, reveal expands, `collapsed` saved in layout); verify store, rendering and mapping tests, including that validation and generation inputs are unchanged

## 3. Editing feel and keyboard

- [x] 3.1 Grid and neighbour snapping with guide lines, a toolbar toggle remembered per browser, and Alt to suspend; verify drag tests for grid snap, guide snap and Alt
- [x] 3.2 Keyboard navigation from one shortcut table: Tab / Shift+Tab in reading order, Ctrl+Tab to transitions, arrows and Shift+arrows move with one undo per burst, F2 rename, `+` / `-` zoom, visible focus ring, `?` help listing the table; verify component tests for each shortcut and that the help lists every entry

## 4. Docs and checks

- [x] 4.1 Document navigation and the shortcut table in `06-ui.md` (Canvas tools) and note which part of NFR-004 this meets; run `ng lint`, `ng test`, `ng build` (no style budget error) and verify all pass
