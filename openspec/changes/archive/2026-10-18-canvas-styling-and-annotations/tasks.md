# Tasks

## 1. Style model and persistence

- [x] 1.1 Add `ElementStyle` and `style?` to `CanvasNode` and `CanvasEdge`, map the legacy node `color` to `style.stroke` everywhere it is read (canvas, panel, quick bar, `colorSelection`), and add store operations `setStyle(ids, key, value)` that checkpoint undo once for any selection; verify store tests for single, bulk, "Default" (key removed) and one-step undo
- [x] 1.2 Read and write `style` in `layout-doc.ts` for states and transitions (non-defaults only, unknown values ignored), mirroring `style.stroke` into `states[id].color`; verify round-trip tests, the legacy `color` read, and that unknown style values fall back to defaults

## 2. Rendering styles

- [x] 2.1 Apply stroke, dash (scaled by width), width, fill and text style to state shapes and labels, with light text on dark fills unless a text colour is set; verify canvas tests for each key and for the light-text rule
- [x] 2.2 Apply line style and text style to transitions and their guard text in `canvas-edges`, and generate deduplicated arrowhead markers per `(kind, colour)` for filled, open and line heads; verify component tests that a dashed or coloured edge renders the right attributes and that two edges with the same colour share a marker
- [x] 2.3 Make the coverage, path-highlight, simulation and selection cues win over user styles (cue classes own stroke, dash and width), and verify component tests that an uncovered dashed blue transition shows the coverage cue and returns to its style when the overlay is off

## 3. Shapes and sizing

- [x] 3.1 Create `state/node-shapes.ts` with outline paths for all seven shapes, add the four new shapes to `NodeShape`, the size and text-width tables, and the parallelogram anchor inset; verify unit tests that each path fits its box and that each anchor lies on the outline
- [x] 3.2 Render the new shapes on the canvas, in the Properties panel shape picker and in the context menu, falling back to the kind's default shape for unknown layout values; verify canvas and panel tests and a layout test for an unknown shape
- [x] 3.3 Pass the label's text style into `node-fit.ts` measurement and sizing (font size, weight, italic, scaled line height); verify unit tests that large or bold labels grow the state and that normal never shrinks it below the shape's default
- [x] 3.4 Update `06-ui.md` with shapes, styles and status-cue precedence, and the "What the editor saves" list; verify the doc names every control the panel offers

## 4. Properties panel styling

- [x] 4.1 Create `components/style-section/` with line pattern, width, line colour, fill, arrowhead and text controls (preset palette, "Default", native custom colour picker), showing only the keys that apply to the selection and a mixed state for differing values; verify component tests for applicable keys per element mix, mixed display and custom colour
- [x] 4.2 Host the style section in the Properties panel for a single state, a single transition and multi-selections (including transitions), replacing the panel's border-colour rows, and hide it in present mode; verify panel tests that a bulk change hits every selected element and that `properties-panel.scss` did not grow

## 5. Annotations

- [x] 5.1 Add `CanvasAnnotation`, the `annotations` signal, the `'annotation'` selection type, and store operations (add, update text with the 2,000-character cap, move, resize, delete, remove when empty) included in undo snapshots, marquee selection, alignment, snapping and delete-selection; verify store tests including the cap, empty removal and undo
- [x] 5.2 Read and write `layout.annotations` in `layout-doc.ts`, dropping malformed entries; verify round-trip tests and that a layout without the key loads with no annotations
- [x] 5.3 Create `components/canvas-annotations/` to render notes (folded corner, default pale yellow) and text boxes with wrapped text, auto-height notes, resize handles, and the in-place editor (double-click or F2, Shift+Enter for a line break, Enter commits, Escape cancels), read-only in present mode; verify component tests for wrapping on resize, editing keys and present mode
- [x] 5.4 Add note and text tools to the quick bar palette (drop opens the editor) and replace the quick bar's border-colour row, and make transition drops on an annotation create nothing; verify canvas tests for drop-to-create and for no transition to a note
- [x] 5.5 Show annotation text and style controls in the Properties panel via the style section; verify a panel test that editing a note's text and fill updates it
- [x] 5.6 Verify that annotations stay out of the model: tests that `toRemote`/graph mapping, search, validation input and AI mapping contain no annotation data, and a backend test that `remap_layout_ids` leaves annotation ids and text unchanged on duplicate; document annotations in `06-ui.md`

## 6. Integration

- [x] 6.1 Run `ng lint`, `ng test` and `ng build` (no component style budget errors), plus `cargo test -p tm-domain`, and verify all pass
- [ ] 6.2 Manually check in the running app: style a state, a transition and a selection, add a note, reload, duplicate the model, and switch on coverage and present mode; verify everything matches the specs
