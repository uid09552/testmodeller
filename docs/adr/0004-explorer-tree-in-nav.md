---
title: 0004. The explorer tree lives in the nav bar
type: adr
status: accepted
date: 2026-10-05
tags: [ui, navigation]
related: [../specification/06-ui.md]
---

# 0004. The explorer tree lives in the nav bar

## Context

The Project > Component > Feature > Model tree was a panel on the Explorer
page. Reaching a model meant navigating to Explorer, expanding the tree,
clicking the model to see an overview, and then opening the editor — and the
tree disappeared as soon as the editor opened.

## Decision

The tree is the left nav bar's main content, headed "Projects", and is
therefore available on every screen. It has no nav item of its own — an item
that only revealed what is already permanently on screen was one control too
many — and the cross-cutting views stay listed below it. Selection state moves
from the Explorer page component into `ExplorerStore` so the tree and the
Explorer page agree on what is selected.

Single-clicking a model opens the model editor. The model overview — both the
Explorer page's model detail pane and the unrouted model list page — is
removed, because it only linked on to the editor.

## Consequences

- One tree instance, one selection state; the Explorer page is a detail view
  for projects, components and features only.
- The expanded nav bar is wider than a list of labels would need, and
  collapsing it to icons is the only way to hide the tree.
- Nothing links to the Explorer page from the nav any more; it is reached by
  selecting a project, component or feature in the tree.
- A model can no longer be inspected without opening it. The tree row keeps the
  status dot, and the editor shows states, transitions and errors in its
  Properties tab, so nothing that pane showed is lost.
