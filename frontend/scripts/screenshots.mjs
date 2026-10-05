/**
 * Captures the screenshots used in the documentation.
 *
 *   cd frontend && npm run screenshots      (or: make screenshots)
 *
 * Serves the production build on a free port with a small stub of the API, so
 * the shots show the UI in a realistic state without a backend or a database,
 * then drives Chromium through the screens and writes PNGs to
 * docs/screenshots/.
 *
 * Re-run it after a UI change so the documentation does not drift. It needs a
 * production build first (`make build-fe`) and Playwright's Chromium.
 */
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { extname, join, resolve } from 'node:path';
import { chromium } from 'playwright';

const ROOT = resolve(import.meta.dirname, '../..');
const DIST = join(ROOT, 'frontend/dist/testmodeller-ui/browser');
const OUT = join(ROOT, 'docs/screenshots');

const VIEWPORT = { width: 1440, height: 900 };

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
};

/**
 * Just enough of the API for the screens that call it.
 *
 * Kept deliberately small: these are canned answers for documentation, not a
 * second implementation of the backend.
 */
const API_STUB = {
  'GET /api/v1/settings/ai': {
    provider: 'anthropic',
    model: 'claude-opus-5-5',
    secretConfigured: true,
    maxTokensPerRequest: 16000,
  },
  'GET /api/v1/health': { status: 'ok', version: '0.1.0' },
};

function startServer() {
  const server = createServer(async (req, res) => {
    const url = new URL(req.url, 'http://localhost');
    const stub = API_STUB[`${req.method} ${url.pathname}`];
    if (stub) {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify(stub));
      return;
    }
    if (url.pathname.startsWith('/api/')) {
      res.writeHead(404, { 'content-type': 'application/problem+json' });
      res.end(JSON.stringify({ title: 'Not Found', status: 404 }));
      return;
    }
    // Static file, with an SPA fallback so deep links resolve.
    const candidate = join(DIST, url.pathname);
    const file = candidate.startsWith(DIST) && existsSync(candidate) && extname(candidate)
      ? candidate
      : join(DIST, 'index.html');
    try {
      const body = await readFile(file);
      res.writeHead(200, { 'content-type': MIME[extname(file)] ?? 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(500);
      res.end('error');
    }
  });
  return new Promise(ok => server.listen(0, '127.0.0.1', () => ok(server)));
}

/** Pause for layout, fonts and the canvas to settle before shooting. */
async function settle(page) {
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(400);
}

async function shoot(page, name) {
  await settle(page);
  await page.screenshot({ path: join(OUT, `${name}.png`) });
  console.log(`  wrote docs/screenshots/${name}.png`);
}

async function main() {
  if (!existsSync(DIST)) {
    console.error(`No production build at ${DIST}. Run: make build-fe`);
    process.exit(1);
  }
  await mkdir(OUT, { recursive: true });

  const server = await startServer();
  const base = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: VIEWPORT });
  const page = await context.newPage();

  try {
    // ── Projects tree and the explorer detail ──────────────────────────────
    await page.goto(`${base}/explorer`);
    await settle(page);
    // The demo tree seeds itself on first run. Clicking a row also toggles it,
    // so the feature is clicked twice: selected, and still expanded.
    const feature = page.getByRole('treeitem', { name: 'Feature Login' });
    await feature.click();
    await feature.click();
    await shoot(page, '01-projects-tree');

    // ── Model editor, on its default Test Cases tab ────────────────────────
    await page.getByRole('treeitem', { name: 'Open model Login Flow' }).click();
    await settle(page);
    await shoot(page, '02-model-editor');

    // ── Test cases of a selected state ─────────────────────────────────────
    // Double-clicking a state's chips is the shortcut the UI advertises; the
    // Login state is the one with cases in more than one category.
    await page
      .getByRole('button', { name: 'regular state: Login' })
      .locator('.node-tests')
      .dblclick();
    await shoot(page, '03-test-cases');

    // ── Properties of the selected state ───────────────────────────────────
    await page.getByRole('tab', { name: 'Properties' }).click();
    await shoot(page, '04-properties');

    // ── AI assistant ───────────────────────────────────────────────────────
    await page.getByRole('tab', { name: 'AI' }).click();
    await shoot(page, '05-ai-assistant');

    // ── Cross-cutting views ────────────────────────────────────────────────
    await page.goto(`${base}/test-cases`);
    await shoot(page, '06-test-case-list');

    await page.goto(`${base}/coverage`);
    await shoot(page, '07-coverage');

    await page.goto(`${base}/settings`);
    await shoot(page, '08-settings');
  } finally {
    await browser.close();
    server.close();
  }
}

await main();
