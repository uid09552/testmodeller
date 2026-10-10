# Design

## Context

States carry `shape` (`circle | rect | diamond`) and `color`, which the canvas uses as the border colour although the type comment says "fill override". Transitions carry routing data only. Their look comes from `canvas-edges.scss` and three shared `<marker>`s in `canvas.html` (`arrow`, `arrow-selected`, `arrow-drawing`). Status cues are CSS classes on the same elements: coverage uses `edge-group--uncovered` with a dash, path highlight uses `--on-path`/`--dimmed`, and simulation uses `--sim-enabled`/`--sim-blocked`. `node-fit.ts` sizes states from one label font (`500 LABEL_FONT_PX`) with a per-shape `MAX_TEXT_W` and `INNER_FRAC`. The layout document (`layout-doc.ts`, ADR 0010) stores only non-defaults and ignores unknown keys. The backend's `remap_layout_ids` swaps every whole-string UUID found in the map, which covers only graph ids. The selection is `{id, type: 'node' | 'edge' | 'group'}[]`, and undo snapshots nodes, edges and groups. Component styles have a 14 kB error budget, and `canvas.scss` and `properties-panel.scss` are already large. See proposal.md for the why, and the specs for the behaviour.

## Goals / Non-Goals

**Goals:**
- One style model shared by states, transitions and annotations, so the panel, bulk edits and persistence handle all three with the same code.
- Status cues stay correct and readable whatever the user styles.
- No contract change and no layout version bump.

**Non-Goals:**
- Connectors from annotations to states (UML "note attached" dashed lines). This is a natural follow-up, but it needs anchoring rules of its own.
- Copy/paste of elements or styles (a format painter), and named style presets.
- Free shapes with no model meaning other than notes and text, such as plain rectangles, images or icons. Groups already cover framing.
- Theme-aware user colours. A user colour is shown as chosen in light and dark mode. Defaults keep using theme tokens.

## Decisions

- **Style object.** A single optional `style?: ElementStyle` on `CanvasNode`, `CanvasEdge` and `CanvasAnnotation`:
  `{ stroke?, dash?: 'dashed' | 'dotted', width?: 1|2|3|4, fill?, text?, size?: 's' | 'l', bold?, italic?, arrow?: 'open' | 'line' }`. An absent key means default, so "Default" in a control deletes the key. The existing state `color` becomes `style.stroke` in memory. The layout reader accepts both, and the writer stores `style` and keeps mirroring `style.stroke` into `states[id].color`, so a previous frontend still shows border colours. Alternative: flat fields per element type. Rejected because it would mean three copies of the panel code and three bulk-edit paths.
- **Rendering.** Styles are applied as SVG attributes and inline style (`stroke`, `stroke-width`, `stroke-dasharray` scaled by width, `fill`, `font-*`) on the existing elements, so the CSS defaults still apply when a key is absent. Status cue classes use `!important` on the properties they own (stroke, dash, width). That keeps cues above user styles without per-cue template logic. The selection outline is a separate element and is unaffected.
- **Arrowheads.** `<marker>`s cannot inherit the path's stroke colour in all target browsers (`context-stroke` is not reliable). So the edges component emits one marker per distinct `(arrow kind, colour)` pair in use, keyed by a short hash, inside its own `<defs>`. Defaults keep the existing three markers. Alternative: draw the arrowhead as a path at the end point. Rejected because it needs end-tangent math for every routing style, which markers already get for free.
- **Shapes.** `NodeShape` gains `hexagon | parallelogram | cylinder | document`, with `SIZE_FOR_SHAPE`, `MAX_TEXT_W` and `INNER_FRAC` entries, and outline paths in a small pure module `state/node-shapes.ts` (`shapePath(shape, w, h)`) that the canvas, the palette icons and tests share. Connector points stay at the bounding-box midpoints. Where the outline does not touch a midpoint (the parallelogram's left and right sides, which are inset by the skew), `anchorPoint` gets a per-shape inset, so transitions end on the outline. This is the only geometry change.
- **Font in sizing.** `measureLabel` and `fitNodeSize` take a `TextStyle` (size in px, weight, italic). The measure cache is keyed by the font string. Sizes: small 11 px, normal the current `LABEL_FONT_PX`, large 16 px, with line height scaled accordingly.
- **Annotations as their own collection.** `annotations = signal<CanvasAnnotation[]>` with `{ id, kind: 'note' | 'text', x, y, w, h, text, style? }`, and the selection type gains `'annotation'`. They are stored as `layout.annotations: [...]` (an array, like `groups`). Keeping them out of `nodes` guarantees by construction that validation, generation, AI mapping, the table and search never see them: none of those read the layout. Alternative: a state kind `note`. Rejected because every graph consumer would need a filter, and the backend would store notes as states. Ids are `crypto.randomUUID()`. The backend remap leaves them unchanged, which is correct because they never refer to graph elements.
- **Annotation rendering** lives in a new `components/canvas-annotations/` attribute component inside the canvas SVG, like `canvas-edges`, drawn above groups and below transitions. Text is wrapped with the same `labelLines` measurer at the annotation's width. A note's height is `max(h, fitted text height)`. The in-place editor reuses the state editor's `foreignObject` and textarea pattern.
- **Undo and drags** reuse the existing pattern: a discrete style change calls `checkpoint()` once and then patches every selected element, and annotation drags and resizes checkpoint on mouse-down. Snapshots include `annotations`.
- **Properties panel.** A new `components/style-section/` component takes the current selection's elements and their applicable keys, and emits `(key, value)`. The panel hosts it for single and multiple selections. "Applies to all" is the intersection of keys per element type (state: all but `arrow`; transition: all but `fill`; note: all but `arrow`; text: text keys only). A mixed value is shown with `aria-checked="mixed"` on radio groups and an empty colour chip labelled "Mixed".
- **Colour control.** The preset row plus a native `<input type="color">` behind a "Custom" swatch. Native avoids a dependency and is accessible by keyboard.
- **Quick bar and context menu.** The quick bar's state border colour row is replaced by the palette's two annotation tools (note, text). Colour stays in the Properties panel and the multi-select bar. The context menu's shape submenu lists all seven shapes.

## Risks / Trade-offs

- [Many custom colours create many markers] → Markers are deduplicated by `(kind, colour)` and only in-use pairs are emitted. That is bounded by the number of transitions and negligible in practice.
- [Layout growth from notes] → Text is capped at 2,000 characters, and only non-default style keys are stored. At about 2.2 KB per note, the 256 KB limit is far beyond realistic use, and an over-limit save already fails with a visible validation error.
- [`!important` in cue classes is blunt] → It is confined to the cue classes in `canvas-edges.scss` and the node cue classes, and documented there. Cues are meant to win.
- [Older editors drop new data] → An older editor reading a v1 layout ignores `style` and `annotations`, and its next save would drop them. Accepted: every user gets the same frontend build. Readers still accept the legacy `color`.
- [Style budget] → New styles go into the new components. `canvas.scss` and `properties-panel.scss` must not grow, and the `ng build` budget check catches it if they do.
- [Dark-mode contrast of user colours] → Light text switches automatically on dark fills (the existing `needsLightText`). Other contrast is the user's choice.

## Migration Plan

No data migration. On read, the legacy `states[id].color` maps to `style.stroke`. On save, both are written. Rolling back to a previous frontend keeps border colours, because `color` is still written, and loses only the new keys on that model's next save.
