---
title: Angular coding guide
type: guide
status: active
tags: [angular, frontend, conventions]
related: [testing-guide.md, workflow.md]
---

# Angular coding guide

- Latest stable Angular, TypeScript `strict` on.
- Standalone components only; `ChangeDetectionStrategy.OnPush`.
- State with signals; RxJS only for streams (HTTP, events). Avoid manual subscriptions; use `toSignal`/`async`.
- `inject()` over constructor injection.
- Folders: `core/`, `shared/`, `features/<name>/{pages,components,services,state}`.
- Feature routes lazy-loaded.
- Typed API services in `core/api`; no `any`.
- Templates: new control flow (`@if`, `@for`), no logic-heavy expressions.
- Styling: SCSS with design tokens (CSS variables); no inline styles.
- Accessibility: semantic HTML, labels, focus management, keyboard support.
- Tests: Jest or Karma per ADR; test components via harnesses; mock API at service level.
- Lint: ESLint with `@angular-eslint`; Prettier for formatting.

## Component style budgets

`angular.json` sets `anyComponentStyle` to warn at 8 kB and fail at 14 kB
(raised from 4/8 on 2026-10-04).

Before raising it, the shared parts were extracted into `src/styles/_ui.scss`,
which is emitted once globally instead of being inlined per component:

- page/layout primitives, buttons, cards, tables, forms, stat tiles
- context menus and modal dialogs (the canvas and the explorer had near-identical copies)
- colour swatches, shape pickers, badges, category/polarity pills

**Do not put plain CSS rules in a partial that components `@use`.** `@use`
inlines them into every consumer, so a 7 kB partial used by eight components
costs 56 kB and trips the budget in eight places at once. `styles/_page.scss`
therefore contains only a mixin; shared classes belong in `styles/_ui.scss`,
which only the global `styles.scss` pulls in.

What remains over 8 kB is genuinely component-specific: the model canvas is an
SVG diagram editor with a toolbar, a shape palette, four node shapes, edge
routing, endpoint handles and connector dots. If a component approaches 14 kB,
extract shared rules or split the component — do not raise the budget again
without checking for duplication first.
