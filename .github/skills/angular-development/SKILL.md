---
name: angular-development
description: Angular UI development for TestModeller. Use when adding or changing code in frontend/, components, state, routing, scenario editor or test case views.
---

# Angular development

1. Read `docs/guides/angular-coding-guide.md` and `docs/specification/06-ui.md`.
2. Structure: `src/app/{core,shared,features/<feature>}`; lazy-loaded feature routes.
3. Standalone components, `OnPush`, signals for state, `inject()` for DI.
4. API access only via typed services in `core/api`.
5. Scenario editor is a graph/state-machine canvas; keep editor logic in a separate service from rendering.
6. Verify: `ng lint && ng test --watch=false && ng build`.
