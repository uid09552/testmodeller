# Proposal

## Why

The model editor shows validation results passively. The toolbar badge ("1 error") looks clickable but does nothing, and finding the offending element means opening the Validation tab, then using a small icon button. Separately, a state's name is a single line: users cannot break a long name where they want, and the automatic sizing only wraps at word boundaries.

## What Changes

- Clicking the error/warning badge in the editor toolbar opens the bottom panel's Validation tab.
- Clicking an issue row (not only its small icon) selects the offending state or transition and brings it into view; the existing button selected transitions as if they were states.
- Inline state editing accepts line breaks (Shift+Enter inserts one; Enter still commits, Escape cancels).
- The Properties panel label field accepts line breaks too.
- State sizing honours explicit line breaks: each line is wrapped on its own, and the state grows to fit all lines.
- Text outside the canvas that shows a state name (validation messages, tree, test references) shows line breaks as spaces.

## Capabilities

### New Capabilities
- `model-validation`: How validation results are surfaced and navigated in the model editor.

### Modified Capabilities
- `canvas-state-nodes`: state labels may contain explicit line breaks, and sizing accounts for them.

## Impact

- `frontend/src/features/models/` — store (panel tab request, reveal request), canvas (badge, inline editor, label rendering), bottom panel (shared tab, issue rows), properties panel (label field), `state/node-fit.ts`.
- `docs/specification/06-ui.md`.
- No API change: state `name` is already a free string with `minLength: 1`, so a stored name may contain a newline. No spec change to `openapi.yaml` is proposed.
