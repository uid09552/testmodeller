# Screenshots

Generated, not captured by hand, so they can be refreshed in one command when
the UI changes:

```bash
make build-fe      # the script serves the production build
make screenshots
```

`frontend/scripts/screenshots.mjs` serves `frontend/dist` on a free port with a
small stub of the API, drives Chromium through the screens, and writes the PNGs
here at 1440×900.

Adding a shot means adding a step to that script, so every image on the
documentation site stays reproducible. Keep the file names numbered in the
order they appear in [the documentation home](../index.md).

Requires Playwright's Chromium:

```bash
cd frontend && npx playwright install chromium
```
