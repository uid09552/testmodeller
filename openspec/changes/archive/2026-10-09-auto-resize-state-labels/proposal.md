# Proposal

## Why

State names on the canvas are drawn as a single SVG text line in a fixed-size shape (rect 144×48, circle 88×88, diamond 172×104). A long name overflows the shape and overlaps neighbouring states, transitions and chips, making the diagram unreadable.

## What Changes

- A state automatically grows so its label fits inside it; no manual resizing.
- Growth applies when a state is created, renamed (including while typing), or loaded, for all shapes.
- States never shrink below their shape's default size; shortening a name shrinks the state back toward that default.
- Very long names wrap onto multiple lines up to a maximum width, then the state grows in height.
- Connectors, test chips, groups and edge endpoints follow the new size.

## Capabilities

### New Capabilities
- `canvas-state-nodes`: How states are sized and labelled on the model canvas, starting with automatic fitting of long labels.

### Modified Capabilities

## Impact

- `frontend/src/features/models/state/model-editor.store.ts` (sizing logic, `updateNode`), `components/canvas/` (label rendering, inline edit box).
- Persisted node width/height (`w`/`h`) may change on save; no API schema change is expected (width/height already persisted) — to be confirmed against `openapi.yaml` during apply. No API spec change is proposed.
- `docs/specification/06-ui.md` gains the sizing rule.
