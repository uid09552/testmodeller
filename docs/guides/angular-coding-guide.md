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
