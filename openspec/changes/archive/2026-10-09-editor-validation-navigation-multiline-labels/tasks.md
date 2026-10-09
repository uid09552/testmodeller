# Tasks

## 1. Validation navigation

- [x] 1.1 Move the bottom panel's active tab into the store and make the toolbar badge a button (only when issues exist) that opens the Validation tab; verify canvas/bottom-panel tests show clicking "1 error" and pressing Enter/Space show the Validation tab, and the valid indicator does nothing
- [x] 1.2 Make whole issue rows navigate: select the element with its real type (node or edge) and reveal it if outside the viewport; verify tests for a state issue, a transition issue, and a row without an element
- [x] 1.3 Add the behavior to `docs/specification/06-ui.md`; verify it matches the spec scenarios

## 2. Multi-line state names

- [x] 2.1 Make `labelLines`/`fitNodeSize` honour explicit line breaks; verify `node-fit` unit tests cover stacked short lines, a break plus a long line, and blank lines
- [x] 2.2 Replace the inline editor with a textarea (Shift+Enter breaks, Enter commits, Escape cancels, trailing blank lines dropped) that fits the resizing node; verify canvas tests for each key and live resize
- [x] 2.3 Make the Properties panel label a textarea and normalise line breaks to spaces in validation messages, the explorer tree and test references; verify tests, including a model-mapping round trip of a multi-line name
- [x] 2.4 Document multi-line names in `docs/specification/06-ui.md`; confirm `openapi.yaml` needs no change

## 3. Checks

- [x] 3.1 Run `ng lint` and `ng test`; verify both pass
