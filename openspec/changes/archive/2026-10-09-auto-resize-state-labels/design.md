# Design

## Context

`SIZE_FOR_SHAPE` gives fixed `w`/`h` per shape; `updateNode` resets size only when the shape changes. The canvas renders `node.label` as one `<text>` centred in the shape, so overflow is unclipped. Anchors, edges, test chips and groups derive from `node.w`/`node.h`, so changing those values propagates everywhere. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Automatic fit for all shapes, live while editing.
- Deterministic, testable sizing function.

**Non-Goals:**
- Manual drag-resize handles.
- Auto-resizing groups' labels or transition labels.
- Re-layout/collision avoidance of neighbouring states.

## Decisions

- **Compute size from measured text width** via a pure function `fitNodeSize(label, shape)` in the store layer, using canvas `measureText` with the label font (13px, weight 500), cached per label. Alternative: SVG `getBBox` after render — rejected: forces layout in the render loop and is hard to unit test. Estimated char widths — rejected as inaccurate.
- **Wrap by words at a maximum text width** (rect ≈ 240px, circle/diamond scaled by their usable inner area), rendered as `<tspan>` lines; height grows by line count. Shape default size is the minimum.
- **Fit inside the shape's inner area**: rects use full width minus padding; ellipses and diamonds use the inscribed rectangle (≈ 70% / 50% of w) so text clears curved or angled edges.
- **Resize around the centre** so neighbours and connections do not appear to jump.
- **Apply in `updateNode`** whenever `label`/`shape` changes, and once when loading a model; `w`/`h` stay the stored fields, so persistence and the API are untouched.
- The undo rule is unchanged: label typing does not push undo, so resize rides along with it.

## Risks / Trade-offs

- [Font not loaded at measurement time gives wrong widths] → Re-fit after `document.fonts.ready`.
- [Grown states overlap neighbours] → Accepted; overlap now comes from layout, not text. Align/auto-layout is out of scope.
- [Saved sizes change for existing models on load] → Sizing only grows past the default, so unchanged models look the same.
