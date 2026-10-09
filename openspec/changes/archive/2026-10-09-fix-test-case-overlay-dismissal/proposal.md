# Proposal

## Why

The test case overlay sometimes closes on its own while the user is working in it, discarding unsaved edits. The overlay's backdrop treats any click that lands on it as "cancel", and a click event fires on the backdrop whenever a press starts inside the dialog (e.g. selecting text in a textarea) and the pointer is released outside it. Escape also dismisses the overlay with no confirmation. Work in progress should only be abandoned by an explicit action.

## What Changes

- Clicking (or press-drag-releasing) on the backdrop no longer closes the test case overlay.
- The overlay closes only through: Save, Cancel (footer), or the close (X) button in the top-right corner.
- The Escape key no longer closes the overlay (assumption: "only" is meant literally; see design.md).
- Applies to every place the overlay is used (model editor right panel, bottom panel, Test Cases list page), since they share one component.

## Capabilities

### New Capabilities
- `test-case-editor`: Behavior of the Gherkin test case overlay editor, starting with when it may be dismissed.

### Modified Capabilities

## Impact

- `frontend/src/features/models/components/test-case-dialog/` (template, component, spec).
- `docs/specification/06-ui.md` gains the dismissal rule.
- No API, backend or dependency changes.
