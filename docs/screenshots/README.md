# Screenshots

Generated, not captured by hand, so they can be refreshed in one command when
the UI changes:

```bash
make build-fe                          # the production build the script serves
(cd backend && cargo build --bin testmodeller)
make screenshots
```

`frontend/scripts/screenshots.mjs` is end to end. It

1. starts the backend binary in `--dev-mode` on a free port (a throwaway
   Postgres in Docker; the first run pulls the image),
2. seeds a small *Webshop* project through the public API: a feature with a
   model and test cases, and a threat model with zones, components, assets,
   flows, threats, controls with evidence, security tests and a baseline,
3. serves `frontend/dist` with `/api` proxied to that backend,
4. drives Chromium through the screens and writes the PNGs here at 1440×900,
5. stops the backend, which removes the database container.

Nothing is mocked, so a screenshot can only show what the product does. The AI
provider is a placeholder key that is never called: the AI shots show the
panels, not an answer.

Adding a shot means adding a step to that script, and anything it needs to show
goes into the seed, so every image on the documentation site stays
reproducible. Keep the file names numbered in the order they appear in
[the documentation home](../index.md):

| Files | Screen |
| --- | --- |
| `01`–`08` | Projects tree, model editor, test cases, properties, AI assistant, test case list, coverage, settings |
| `09`–`17` | Threat model: overview, diagram, components, threats, risks, controls, security tests, baselines, report |
| `18` | The threat model's AI assistant |

Requires Docker and Playwright's Chromium:

```bash
cd frontend && npx playwright install chromium
```

The coverage screenshot shows placeholder figures, because that page is not
wired to the API yet (see [Status](../index.md#status)).
