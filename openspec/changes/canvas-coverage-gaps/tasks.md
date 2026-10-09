# Tasks

## 1. Data

- [ ] 1.1 Add store state for the overlay mode and for transition coverage fetched from `GET /models/{id}/coverage` (on enable and after save); verify store tests cover states live, transitions as-of-save, and a never-saved model
- [ ] 1.2 Compute covered/total for states and transitions; verify unit tests for empty models, all covered and none covered

## 2. Canvas

- [ ] 2.1 Add the toolbar toggle and dimension choice with counts and the "as of last save" label; verify a canvas test for each mode
- [ ] 2.2 Draw uncovered states and transitions with a dashed outline, marker and accessible label; verify rendered classes and aria-labels in a canvas test
- [ ] 2.3 Selecting an uncovered state opens its Test Cases tab ready to add; verify with a test

## 3. Docs and checks

- [ ] 3.1 Describe the overlay in `docs/specification/06-ui.md`; confirm `openapi.yaml` needs no change
- [ ] 3.2 Run `ng lint` and `ng test`; verify both pass
