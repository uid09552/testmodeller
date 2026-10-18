/**
 * Captures the screenshots used in the documentation.
 *
 *   make screenshots            (or: cd frontend && npm run screenshots)
 *
 * The UI keeps everything in the backend (ADR 0008), so the shots are taken
 * against the real thing: the script starts the backend binary in `--dev-mode`
 * (a throwaway Postgres in Docker), seeds a small shop project through the
 * public API, serves the production build with `/api` proxied to the backend,
 * and drives Chromium through the screens. Nothing here is mocked, so a shot
 * can only show what the product really does.
 *
 * Needs, before it runs:
 *   - `make build-fe`                       the production build it serves
 *   - `cargo build --bin testmodeller`      the backend binary
 *   - Docker                                for the dev database
 *   - `npx playwright install chromium`
 *
 * The AI provider is a placeholder key and is never called: the AI shots show
 * the panels, not an answer.
 */
import { createServer, request as httpRequest } from 'node:http';
import { createServer as createNetServer } from 'node:net';
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { readFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { extname, join, resolve } from 'node:path';
import { chromium } from 'playwright';

const ROOT = resolve(import.meta.dirname, '../..');
const DIST = join(ROOT, 'frontend/dist/testmodeller-ui/browser');
const BACKEND = join(ROOT, 'backend/target/debug/testmodeller');
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

// ── Backend ──────────────────────────────────────────────────────────────────

function freePort() {
  return new Promise((ok, fail) => {
    const s = createNetServer();
    s.once('error', fail);
    s.listen(0, '127.0.0.1', () => {
      const { port } = s.address();
      s.close(() => ok(port));
    });
  });
}

async function startBackend() {
  if (!existsSync(BACKEND)) {
    console.error(`No backend binary at ${BACKEND}. Run: cd backend && cargo build --bin testmodeller`);
    process.exit(1);
  }
  const port = await freePort();
  const child = spawn(BACKEND, ['--dev-mode', '--bind', `127.0.0.1:${port}`], {
    env: {
      ...process.env,
      TM_AI_PROVIDER: 'anthropic',
      TM_AI_MODEL: 'claude-opus-5-5',
      TM_AI_API_KEY: 'sk-ant-documentation-placeholder',
    },
    stdio: ['ignore', 'ignore', 'inherit'],
  });
  const api = `http://127.0.0.1:${port}/api/v1`;
  for (let i = 0; i < 240; i++) {
    if (child.exitCode !== null) throw new Error('the backend exited while starting');
    try {
      if ((await fetch(`${api}/health`)).ok) return { child, api, port };
    } catch {
      /* not up yet */
    }
    await new Promise(r => setTimeout(r, 500));
  }
  child.kill();
  throw new Error('the backend did not become healthy in 2 minutes');
}

async function stopBackend(child) {
  if (child.exitCode !== null) return;
  // SIGTERM lets the backend remove its dev database container.
  await new Promise(done => {
    child.once('exit', done);
    child.kill('SIGTERM');
    setTimeout(() => {
      child.kill('SIGKILL');
      done();
    }, 20_000).unref();
  });
}

// ── Seeding ──────────────────────────────────────────────────────────────────

function client(api) {
  const call = async (method, path, body, version) => {
    const res = await fetch(`${api}${path}`, {
      method,
      headers: {
        ...(body === undefined ? {} : { 'content-type': 'application/json' }),
        ...(version === undefined ? {} : { 'if-match': `"${version}"` }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await res.text();
    if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${text}`);
    return text ? JSON.parse(text) : null;
  };
  return {
    get: p => call('GET', p),
    post: (p, b = {}) => call('POST', p, b),
    put: (p, b, version) => call('PUT', p, b, version),
  };
}

const today = (offsetDays = 0) =>
  new Date(Date.now() + offsetDays * 86_400_000).toISOString().slice(0, 10);

/** A shop with one modelled feature and a threat model around it. */
async function seed(base) {
  const c = client(base);

  // The model-based testing side ------------------------------------------
  const project = await c.post('/projects', {
    name: 'Webshop',
    description: 'Online shop: catalogue, checkout and customer accounts.',
  });
  const accounts = await c.post(`/projects/${project.id}/components`, { name: 'Accounts' });
  const checkout = await c.post(`/projects/${project.id}/components`, { name: 'Checkout' });
  const login = await c.post(`/components/${accounts.id}/features`, {
    name: 'Login',
    scenarioDescription: 'A customer signs in with email and password; repeated failures lock the account.',
    tags: ['auth'],
  });
  await c.post(`/components/${checkout.id}/features`, { name: 'Payment' });

  const ids = Object.fromEntries(['start', 'login', 'auth', 'home', 'locked'].map(k => [k, randomUUID()]));
  const st = (key, name, kind, x, y) => ({ id: ids[key], name, kind, position: { x, y } });
  const modelDoc = {
    name: 'Login Flow',
    description: 'Sign-in with a lock-out after three failed attempts.',
    status: 'ready',
    states: [
      st('start', 'Start', 'initial', 60, 160),
      st('login', 'Login', 'normal', 200, 160),
      st('auth', 'Authenticating', 'normal', 400, 160),
      st('home', 'Account', 'final', 620, 120),
      st('locked', 'Locked', 'final', 620, 215),
    ],
    transitions: [
      { id: randomUUID(), from: ids.start, to: ids.login, event: 'open sign-in page' },
      { id: randomUUID(), from: ids.login, to: ids.auth, event: 'submit credentials' },
      { id: randomUUID(), from: ids.auth, to: ids.home, event: 'credentials valid' },
      { id: randomUUID(), from: ids.auth, to: ids.login, event: 'credentials invalid', guard: 'attempts < 3' },
      { id: randomUUID(), from: ids.auth, to: ids.locked, event: 'credentials invalid', guard: 'attempts == 3' },
    ],
  };
  const model = await c.post(`/features/${login.id}/models`, modelDoc);
  const stateByName = Object.fromEntries(model.states.map(s => [s.name, s.id]));

  const cases = [
    ['Valid credentials open the account', ['feature', 'positive'], 'Login', 'The customer has an account', 'Enter a valid email and password', 'The account page opens'],
    ['Wrong password shows an error', ['feature', 'negative'], 'Login', 'The customer has an account', 'Enter a wrong password', 'The form shows "Invalid email or password"'],
    ['Email is trimmed before lookup', ['unit', 'positive'], 'Login', 'An email with surrounding spaces', 'Normalise the email', 'The spaces are removed'],
    ['Password hash is compared in constant time', ['unit', 'positive'], 'Login', 'A stored password hash', 'Compare two hashes', 'The comparison time does not depend on the content'],
    ['Identity provider answers within two seconds', ['integration', 'positive'], 'Login', 'The identity provider is reachable', 'Send a sign-in request', 'The answer arrives within two seconds'],
    ['Third failed attempt locks the account', ['feature', 'negative'], 'Authenticating', 'The customer failed twice already', 'Enter a wrong password a third time', 'The account is locked'],
    ['Locked account sends a notification mail', ['integration', 'positive'], 'Locked', 'An account was just locked', 'Wait for the lock event', 'A notification mail is queued'],
  ];
  for (const [name, tags, state, given, when, then] of cases) {
    await c.post(`/features/${login.id}/test-cases`, {
      name,
      tags,
      preconditions: given,
      steps: [{ action: when, expected: then }],
      assignments: [{ modelId: model.id, stateId: stateByName[state] }],
    });
  }

  // The threat model -------------------------------------------------------
  const tm = await c.post(`/projects/${project.id}/threat-model`, {
    name: 'Webshop threat model',
    scope: 'The customer-facing shop: browser, API, accounts and payment.',
    objectives: 'Keep customer data and payments safe; keep the shop available.',
    assumptions: 'The cloud provider and the payment provider are trusted.',
    exclusions: 'Physical security of data centres.',
    aiEnabled: true,
  });
  const tmPath = p => `/threat-models/${tm.id}${p}`;

  const zone = (name, kind, trustLevel) => c.post(tmPath('/zones'), { name, kind, trustLevel });
  const zInternet = await zone('Internet', 'internet', 0);
  const zDmz = await zone('DMZ', 'dmz', 2);
  const zInternal = await zone('Internal', 'internal', 3);
  const zData = await zone('Customer data', 'restricted-data', 4);
  const zThird = await zone('Payment provider', 'third-party', 1);

  const asset = (name, category, confidentiality, integrity, availability, personalData, extra = {}) =>
    c.post(tmPath('/assets'), { name, category, confidentiality, integrity, availability, personalData, owner: 'Shop team', ...extra });
  const aCustomers = await asset('Customer records', 'business-data', 'high', 'high', 'medium', true, {
    classification: 'restricted', dataLocation: 'EU (Frankfurt)', retention: '7 years', encryptionAtRest: 'yes',
  });
  const aSessions = await asset('Session tokens', 'identity-secrets', 'high', 'high', 'medium', false);
  const aOrders = await asset('Orders', 'business-data', 'medium', 'high', 'high', true, { classification: 'confidential' });
  const aCards = await asset('Payment references', 'business-data', 'high', 'high', 'low', false, { classification: 'restricted' });

  const comp = (name, type, zone, extra = {}) =>
    c.post(tmPath('/components'), { name, type, zoneId: zone.id, ...extra });
  const browser = await comp('Customer browser', 'human-user', zInternet, { owner: 'Customers' });
  const web = await comp('Shop frontend', 'web-frontend', zInternet, {
    owner: 'Web team', technology: 'Angular', criticality: 'high', internetFacing: true, assetIds: [aSessions.id],
  });
  const gateway = await comp('API gateway', 'api-gateway', zDmz, {
    owner: 'Platform team', technology: 'APISIX', criticality: 'high', internetFacing: true,
  });
  const shopApi = await comp('Shop API', 'api', zInternal, {
    owner: 'Shop team', technology: 'Rust / axum', criticality: 'critical', linkedFeatureId: login.id,
    assetIds: [aCustomers.id, aOrders.id, aSessions.id],
  });
  const idp = await comp('Identity provider', 'identity-provider', zInternal, {
    owner: 'Platform team', technology: 'OIDC', criticality: 'critical',
  });
  const db = await comp('Customer database', 'relational-database', zData, {
    owner: 'Shop team', technology: 'PostgreSQL', criticality: 'critical', assetIds: [aCustomers.id, aOrders.id],
  });
  const audit = await comp('Audit log', 'audit-service', zData, { owner: 'Security team' });
  const pay = await comp('Payment provider', 'third-party-saas', zThird, { criticality: 'high' });
  // One component is deliberately left without owner or zone so the
  // completeness warnings have something to show.
  await c.post(tmPath('/components'), { name: 'Legacy export job', type: 'scheduler' });

  const flow = (src, dst, relationship, label, extra = {}) =>
    c.post(tmPath('/flows'), { sourceId: src.id, targetId: dst.id, relationship, label, ...extra });
  await flow(browser, web, 'calls', 'Browse and sign in', { protocol: 'HTTPS', authentication: 'none', encrypted: 'yes', entryPoint: true });
  const fApi = await flow(web, gateway, 'calls', 'REST calls', {
    protocol: 'HTTPS', authentication: 'unknown', encrypted: 'yes', assetIds: [aSessions.id, aCustomers.id],
    dataClassification: 'confidential', entryPoint: true,
  });
  await flow(gateway, shopApi, 'calls', 'Forwarded requests', {
    protocol: 'HTTP', authentication: 'JWT (RS256)', encrypted: 'unknown', assetIds: [aSessions.id, aCustomers.id],
  });
  await flow(shopApi, idp, 'authenticates-through', 'Verify sign-in', { protocol: 'OIDC', authentication: 'client credentials', encrypted: 'yes' });
  await flow(web, idp, 'authenticates-through', 'Sign-in redirect', { protocol: 'OIDC', authentication: 'none', encrypted: 'yes' });
  await flow(shopApi, db, 'reads-writes', 'Customer and order data', {
    protocol: 'PostgreSQL', authentication: 'password', encrypted: 'no', assetIds: [aCustomers.id, aOrders.id],
  });
  await flow(shopApi, pay, 'sends-data', 'Payment requests', {
    protocol: 'HTTPS', authentication: 'API key', encrypted: 'yes', assetIds: [aCards.id],
  });
  void fApi;
  void audit;

  const manual = async (body) => c.post(tmPath('/threats'), body);
  const tSpoof = await manual({
    title: 'Stolen session token is replayed against the API',
    category: 'spoofing',
    scenario: 'An attacker obtains a session token from a shared computer and replays it.',
    attackerObjective: 'Act as the customer',
    targets: [{ kind: 'component', id: shopApi.id }],
    assetIds: [aSessions.id],
    status: 'open',
    inherent: { likelihood: 4, impact: 4 },
    current: { likelihood: 3, impact: 4 },
    residual: { likelihood: 2, impact: 4 },
    treatment: 'mitigate',
    riskOwner: 'Shop team',
    dueDate: today(21),
    confidence: 'medium',
  });
  const tSql = await manual({
    title: 'Injection through the product search',
    category: 'tampering',
    scenario: 'Crafted search terms reach the database query.',
    targets: [{ kind: 'component', id: shopApi.id }],
    assetIds: [aCustomers.id],
    status: 'open',
    inherent: { likelihood: 3, impact: 5 },
    current: { likelihood: 2, impact: 5 },
    treatment: 'mitigate',
    riskOwner: 'Shop team',
  });
  const tDos = await manual({
    title: 'Checkout flooded with automated orders',
    category: 'denial-of-service',
    targets: [{ kind: 'component', id: gateway.id }],
    status: 'open',
    inherent: { likelihood: 4, impact: 3 },
    current: { likelihood: 4, impact: 3 },
    treatment: 'accept',
    acceptanceRationale: 'Cloud auto-scaling absorbs normal peaks; cost is accepted for now.',
    acceptedBy: 'Head of Engineering',
    acceptanceExpires: today(180),
  });
  void tDos;

  // The manual threats come first, so they head the list; the rules then add
  // their candidates, which stay to be reviewed.
  await c.post(tmPath('/threats/suggest'));

  // Controls, one with evidence so it can be verified.
  const control = (body) => c.post(tmPath('/controls'), body);
  const cTls = await control({
    name: 'Short-lived tokens with rotation',
    type: 'preventive',
    category: 'identity-access',
    status: 'implemented',
    owner: 'Shop team',
    acceptanceCriteria: 'Tokens expire after 15 minutes and refresh tokens rotate on use.',
    componentIds: [shopApi.id, idp.id],
    threatIds: [tSpoof.id],
  });
  const cParam = await control({
    name: 'Parameterised queries everywhere',
    type: 'preventive',
    category: 'secure-development',
    status: 'implemented',
    owner: 'Shop team',
    acceptanceCriteria: 'No query is built by string concatenation; enforced by the linter.',
    componentIds: [shopApi.id],
    threatIds: [tSql.id],
  });
  await c.post(tmPath(`/controls/${cParam.id}/evidence`), {
    kind: 'review', title: 'Code review of the query layer', note: 'All 41 queries use bind parameters.',
    evidenceDate: today(-10), reviewer: 'A. Reviewer',
  });
  const cParamNow = await c.get(tmPath(`/controls/${cParam.id}`));
  await c.put(
    tmPath(`/controls/${cParam.id}`),
    {
      name: cParam.name, type: cParam.type, category: cParam.category, owner: cParam.owner,
      acceptanceCriteria: cParam.acceptanceCriteria, componentIds: [shopApi.id], threatIds: [tSql.id],
      status: 'verified', reviewBy: today(150),
    },
    cParamNow.version,
  );
  await control({
    name: 'Rate limiting at the gateway',
    type: 'preventive',
    category: 'application-network',
    status: 'planned',
    owner: 'Platform team',
    componentIds: [gateway.id],
    threatIds: [],
  });
  void cTls;

  // Security tests from the templates, a few results.
  await c.post(tmPath('/security-tests/generate'), {});
  const list = await c.get(tmPath('/security-tests'));
  for (const [i, t] of list.slice(0, 4).entries()) {
    await c.put(
      tmPath(`/security-tests/${t.id}`),
      {
        title: t.title, objective: t.objective, preconditions: t.preconditions, steps: t.steps,
        type: t.type, controlIds: t.controlIds, threatIds: t.threatIds, status: 'ready',
      },
      t.version,
    );
    await c.post(tmPath(`/security-tests/${t.id}/results`), {
      outcome: i === 1 ? 'failed' : 'passed',
      note: i === 1 ? 'Expired token was still accepted for 30 s.' : undefined,
    });
  }

  await c.post(tmPath('/baselines'), { name: 'Release 1.0', note: 'Reviewed before the first release.' });

  return { projectId: project.id, modelId: model.id, featureId: login.id, tmId: tm.id };
}

// ── Static server with an API proxy ──────────────────────────────────────────

function startServer(backendPort) {
  const server = createServer((req, res) => {
    const url = new URL(req.url, 'http://localhost');
    if (url.pathname.startsWith('/api/')) {
      const upstream = httpRequest(
        { host: '127.0.0.1', port: backendPort, path: req.url, method: req.method, headers: req.headers },
        r => {
          res.writeHead(r.statusCode ?? 502, r.headers);
          r.pipe(res);
        },
      );
      upstream.on('error', () => {
        res.writeHead(502);
        res.end('bad gateway');
      });
      req.pipe(upstream);
      return;
    }
    // Static file, with an SPA fallback so deep links resolve.
    const candidate = join(DIST, url.pathname);
    const file = candidate.startsWith(DIST) && existsSync(candidate) && extname(candidate)
      ? candidate
      : join(DIST, 'index.html');
    readFile(file).then(
      body => {
        res.writeHead(200, { 'content-type': MIME[extname(file)] ?? 'application/octet-stream' });
        res.end(body);
      },
      () => {
        res.writeHead(500);
        res.end('error');
      },
    );
  });
  return new Promise(ok => server.listen(0, '127.0.0.1', () => ok(server)));
}

// ── Shots ────────────────────────────────────────────────────────────────────

/** Pause for layout, fonts and the canvas to settle before shooting. */
async function settle(page) {
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(600);
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

  console.log('starting the backend (first run pulls the Postgres image)...');
  const backend = await startBackend();
  let server;
  let browser;
  try {
    console.log('seeding...');
    const ids = await seed(backend.api);
    server = await startServer(backend.port);
    const base = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch();
    const page = await (await browser.newContext({ viewport: VIEWPORT })).newPage();
    page.on('pageerror', e => console.warn('  page error:', e.message));

    // ── Projects tree and the explorer detail ────────────────────────────
    await page.goto(`${base}/explorer`);
    await settle(page);
    // The tree opens fully expanded. Clicking a row also toggles it, so the
    // feature is clicked twice: selected, and still expanded.
    const feature = page.getByRole('treeitem', { name: 'Feature Login' });
    await feature.click();
    await feature.click();
    await shoot(page, '01-projects-tree');

    // ── Model editor, on its default Test Cases tab ──────────────────────
    await page.goto(`${base}/models/${ids.modelId}`);
    await settle(page);
    await page.getByRole('button', { name: 'Centre the diagram' }).click();
    await shoot(page, '02-model-editor');

    // ── Test cases of a selected state ───────────────────────────────────
    await page
      .getByRole('button', { name: 'regular state: Login' })
      .locator('.node-tests')
      .dblclick();
    await shoot(page, '03-test-cases');

    // ── Properties of the selected state ─────────────────────────────────
    await page.getByRole('tab', { name: 'Properties' }).click();
    await shoot(page, '04-properties');

    // ── AI assistant: a chat window over the canvas ──────────────────────
    await page.getByRole('button', { name: 'AI assistant' }).click();
    await shoot(page, '05-ai-assistant');

    // ── Cross-cutting views ──────────────────────────────────────────────
    await page.goto(`${base}/test-cases`);
    await shoot(page, '06-test-case-list');

    await page.goto(`${base}/coverage`);
    await shoot(page, '07-coverage');

    await page.goto(`${base}/settings`);
    await shoot(page, '08-settings');

    // ── Threat modeling ──────────────────────────────────────────────────
    const tm = tab => `${base}/threat-modeling/${ids.projectId}/${tab}`;
    const tmShots = [
      ['overview', '09-threat-overview'],
      ['diagram', '10-threat-diagram'],
      ['components', '11-threat-components'],
      ['threats', '12-threat-threats'],
      ['risks', '13-threat-risks'],
      ['controls', '14-threat-controls'],
      ['security-tests', '15-threat-security-tests'],
      ['baselines', '16-threat-baselines'],
      ['report', '17-threat-report'],
    ];
    for (const [tab, name] of tmShots) {
      await page.goto(tm(tab));
      if (tab === 'diagram') {
        await settle(page);
        await page.getByRole('button', { name: 'Fit' }).click();
      }
      await shoot(page, name);
    }

    // The AI assistant of the threat model, opened over the Threats tab.
    await page.goto(tm('threats'));
    await settle(page);
    await page.getByRole('button', { name: 'AI assistant' }).click();
    await shoot(page, '18-threat-ai-assistant');
  } finally {
    await browser?.close();
    server?.close();
    await stopBackend(backend.child);
  }
}

await main();
