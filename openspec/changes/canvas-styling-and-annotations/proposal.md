# Proposal

## Why

The editor offers only three shapes and a border colour, so users cannot make a diagram communicate more than its structure. They cannot dash an optional path, set a happy path in a heavier line, colour a group of states, or leave a note next to a tricky branch. Teams coming from draw.io expect these tools and keep drawing a second, "pretty" diagram elsewhere, which then drifts away from the model. Layout storage (ADR 0010) and transition routing are in place, so styling can now be saved with the model without an API change.

## What Changes

- **Line style:** state borders and transitions get a line pattern (solid, dashed, dotted), a line width (1–4 px) and a line colour. The existing state border colour becomes one part of this.
- **Arrowheads:** a transition's arrowhead can be filled (default), open or a thin open "V". A transition always keeps an arrowhead, because direction is part of the model.
- **Fill and text:** states get a fill colour. States, transition labels and annotations get a text colour, font size (small, normal, large), bold and italic. Automatic state sizing takes the font size into account.
- **More state shapes:** hexagon, parallelogram, cylinder and document are added to circle, rectangle and diamond, in the Properties panel and the context menu. A shape is still presentation only and does not change a state's kind.
- **Colours:** every colour control offers the existing palette plus a custom colour picker.
- **Annotations:** new canvas elements with no meaning in the model: a sticky note and a free text box. They can be added from the palette, moved, resized, styled, edited in place, grouped in a selection, deleted and undone. Validation, generation, coverage, simulation, AI and export of test cases ignore them.
- **Multi-select styling:** when several elements are selected, the Properties panel shows the style controls they share. A change applies to all of them as one undo step, and controls whose values differ show a mixed state.
- **Status cues take precedence:** while coverage, path highlight, simulation or selection marks an element, that cue is drawn over the user's style, so a dashed user line cannot hide or imitate a status.
- All styling and annotations are undoable and saved in the layout document (still `v: 1`, new optional keys). **No API change.** Older editors ignore the new keys.

## Capabilities

### New Capabilities
- `canvas-element-styling`: How the user sets the line, fill and text style of states and transitions, singly or for a selection, and how user styles relate to status cues.
- `canvas-annotations`: Notes and text boxes on the canvas that are not part of the model.

### Modified Capabilities
- `canvas-state-nodes`: the set of state shapes grows, and automatic sizing accounts for the label's font size.
- `model-editor-persistence`: the layout that survives a reload includes element styles and annotations.

## Impact

- `frontend/src/features/models/`: store (`CanvasNode`/`CanvasEdge` style fields, a new annotation collection, selection type, bulk style operations with undo), `layout-doc.ts` (new optional keys), `node-fit.ts` (font size, new shapes), canvas rendering of shapes and annotations, `canvas-edges` (dash, width, colour, arrowhead markers), the Properties panel (style section, multi-selection), the palette and context menu.
- `docs/specification/06-ui.md` (styling, annotations, what the editor saves).
- No backend, API or `openapi.yaml` change: the layout is opaque to the backend (ADR 0010). Duplicate and import already remap element ids inside the layout, so annotations need ids that do not collide with state ids.
- `canvas.scss` and `properties-panel` are close to the style budget. Annotation rendering and the style section move into components of their own.
