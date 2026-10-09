# Design

## Context

`tm-test-case-dialog` renders a `.tcd-backdrop` whose `(click)` emits `cancel`, with the inner dialog stopping propagation. A `document:keydown.escape` host listener also emits `cancel`. The component is shared by the model editor panels and the Test Cases list page. Browsers dispatch `click` to the nearest common ancestor of the mousedown and mouseup targets, so a drag that starts in the dialog and ends on the backdrop produces a backdrop click — the "random" close.

## Goals / Non-Goals

**Goals:**
- Dismissal only via Save, Cancel, or the X button.

**Non-Goals:**
- Unsaved-changes confirmation prompt.
- Changing other dialogs (e.g. the delete confirmation `dialog-backdrop`).

## Decisions

- **Remove the backdrop click handler** (and the now-unneeded `stopPropagation` on the dialog) rather than filtering by mousedown target. Simplest, and satisfies "only" literally. Alternative: close only when mousedown and click both hit the backdrop — rejected as it still allows accidental dismissal.
- **Remove the Escape listener.** The request says only those controls close it. Trade-off: loses a keyboard dismissal shortcut; the Cancel and X buttons remain keyboard-focusable. If Escape should stay, it is a one-line retention.
- Keep `aria-modal="true"`; the backdrop still blocks interaction with the page beneath.

## Risks / Trade-offs

- [Users expect Escape/backdrop to close] → Footer Cancel and the X button stay prominent.
