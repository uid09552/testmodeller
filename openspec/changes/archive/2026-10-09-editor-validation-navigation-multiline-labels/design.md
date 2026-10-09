# Design

## Context

The status badge in the canvas toolbar is a non-interactive span fed by `store.errorCount()`/`warningCount()`. The bottom panel keeps its active tab in a local signal, so the sibling canvas cannot change it; both share the page-level `ModelEditorStore`. The issue list's focus button calls `store.select(id, 'node')` for every issue, although issues may concern edges. The inline state editor is an `<input>` inside a `foreignObject`; sizing (`node-fit.ts`) wraps by words and has no notion of `\n`. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Badge opens Validation; issue rows select and reveal the element.
- Multi-line names edited inline and in the Properties panel, with sizing that follows.

**Non-Goals:**
- Quick-fix actions for issues, or new validation rules.
- Multi-line transition labels or group names.
- Any API change.

## Decisions

- **Move the bottom panel's active tab into the store** (`bottomTab` signal) and have the badge set it to `validation`. Alternative: an event output through the page component — more plumbing for the same effect.
- **Reveal via a store signal** (`revealRequest`, element id + counter) that the canvas turns into a pan, only moving the view if the element is outside it. Selection uses the element's real type, looked up from nodes/edges, fixing the edge case.
- **Badge becomes a `<button>`** only when issues exist; the valid indicator stays a span.
- **Line breaks are stored as `\n` in the existing name string.** Alternative: separate field — rejected, it would need an API change. Trade-off: other displays must normalise; a helper collapses `\s*\n\s*` to a space for validation messages, tree and test references.
- **Keys:** Shift+Enter inserts a break, Enter commits. This keeps current muscle memory. Alternative (Enter inserts, Ctrl+Enter commits) was rejected as it changes existing behaviour.
- **Inline editor becomes a `<textarea>`** sized to the node's edit box and growing with the node; rows come from `labelLines`. Committed names are trimmed and trailing blank lines dropped.
- **`labelLines`** splits on `\n` first, then wraps each line at the shape's maximum width; `fitNodeSize` is unchanged beyond consuming that result.
- The Properties panel label input becomes a two-row textarea; Enter there inserts a break (blur commits), as for other multi-line fields in that panel.

## Risks / Trade-offs

- [Names with `\n` shown raw in an unmigrated surface] → grep every `label` display during apply and route through the helper.
- [Backend or export treats newline badly] → name is a free string; add a test that a multi-line name round-trips through the model mapping.
- [Reveal pan fights user zoom/pan] → pan only when the element is out of view, never change zoom.
