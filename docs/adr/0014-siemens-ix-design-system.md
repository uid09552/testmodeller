---
title: 0014. The UI uses the Siemens iX design system
type: adr
status: proposed
date:  # YYYY-MM-DD, the day the decision is accepted
tags: [ui, frontend, design-system]
related: [../../DESIGN.md, ../specification/06-ui.md, ../guides/angular-coding-guide.md]
---

# 0014. The UI uses the Siemens iX design system

## Context

The frontend has a home-grown look: its own colour palette and tokens
(`styles/_tokens.scss`), shared primitives in `styles/_ui.scss`, the Inter
font, and hand-built buttons, dialogs, menus and tables. Nothing tells
contributors or AI agents which components and tokens to use, so styles
drift and get duplicated (see the component style budget history in the
Angular guide). TestModeller targets industrial engineering users, for
whom Siemens Industrial Experience (iX) is the established design language.

## Decision

- Adopt Siemens iX (https://ix.siemens.io/) — `@siemens/ix`,
  `@siemens/ix-angular` (standalone entry point), `@siemens/ix-icons`, and
  `@siemens/ix-aggrid` where AG Grid is used — as the UI's design system.
- `DESIGN.md` at the repository root is the binding guide: component map,
  tokens, layout mapping, button hierarchy, accessibility and a review
  checklist.
- Default theme `community`, colour schema `system`; both dark and light
  must work.
- Enforcement is through the agent instructions: `AGENTS.md` rule 9,
  `CLAUDE.md`, the `frontend-dev` and `reviewer` agents, the
  `angular-development` skill and `angular.instructions.md` all point to
  `DESIGN.md`.
- Migration is incremental: touched components move to iX; no new code uses
  legacy tokens.

## Consequences

- Consistent, accessible, themeable UI with less custom CSS to maintain;
  shared rules in `_ui.scss` shrink over time.
- New runtime dependencies (iX web components on Stencil, plus `luxon` and
  `@floating-ui/dom` as peers) and a larger initial bundle.
- iX majors must be tracked; `DESIGN.md` lists the versions it was written
  against and must be updated on upgrade.
- The Siemens brand theme and Siemens Sans font are licensed separately;
  without them the bundled `community` theme falls back to Helvetica/system
  fonts.
- Until migration finishes, the UI is a mix of legacy and iX styles.
