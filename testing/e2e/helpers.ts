import { expect, type Locator, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

export async function gotoApp(page: Page, path: string) {
  const timeout = process.env.CI ? 90_000 : 45_000;
  // Prod WASM stacks in Docker CI: domcontentloaded can exceed 60s; wait for shell instead.
  if (process.env.CI) {
    await page.goto(path, { waitUntil: 'commit', timeout });
    await page.waitForSelector('nav', { state: 'visible', timeout: 45_000 });
    return;
  }
  await page.goto(path, { waitUntil: 'domcontentloaded', timeout });
  await expect(page.locator('body')).toBeVisible();
}

/** @deprecated Use gotoApp — kept for any external references. */
export async function gotoRouteFast(page: Page, path: string) {
  await gotoApp(page, path);
}

export async function waitForHeading(page: Page, re: RegExp) {
  await expect(page.getByRole('heading', { name: re }).first()).toBeVisible();
}

export async function isOnLoginPage(page: Page) {
  const heading = page.getByRole('heading', { name: /sign in|login/i }).first();
  if (await heading.isVisible().catch(() => false)) return true;
  const email = page.locator('input[type="email"], input[name="email"]').first();
  const pass = page.locator('input[type="password"], input[name="password"]').first();
  return (await email.count()) > 0 && (await pass.count()) > 0;
}

export type E2ECreds = { email: string; password: string };

export type LoginSession = {
  player: Record<string, unknown>;
  session_id: string;
};

export function e2eUserCreds(): E2ECreds | null {
  const email = process.env.E2E_USER_EMAIL;
  const password = process.env.E2E_USER_PASSWORD;
  if (!email || !password) return null;
  return { email, password };
}

export function e2eAdminCreds(): E2ECreds | null {
  const email = process.env.E2E_ADMIN_EMAIL;
  const password = process.env.E2E_ADMIN_PASSWORD;
  if (!email || !password) return null;
  return { email, password };
}

/** API login + localStorage session for an admin user (requires E2E_ADMIN_* env). */
export async function loginAdmin(page: Page): Promise<LoginSession> {
  const creds = e2eAdminCreds();
  if (!creds) {
    throw new Error('E2E_ADMIN_EMAIL and E2E_ADMIN_PASSWORD must be set');
  }
  const session = await applyApiSession(page, creds);
  await expectLoggedIn(page);
  return session;
}

/** Bearer token from gloo_storage (set by applyApiSession / login). */
export async function storedSessionId(page: Page): Promise<string> {
  const raw = await page.evaluate(() => localStorage.getItem('session_id'));
  if (!raw) throw new Error('session_id missing from localStorage');
  return JSON.parse(raw) as string;
}

export function appBaseUrl() {
  return process.env.PLAYWRIGHT_BASE_URL || 'http://127.0.0.1:50003';
}

/** Direct backend for APIRequestContext calls (prod gate sets E2E_BACKEND_URL). */
export function e2eApiBase() {
  return process.env.E2E_BACKEND_URL || appBaseUrl();
}

export function bearerAuth(session_id: string) {
  return { Authorization: `Bearer ${session_id}` };
}

/** Login via direct backend URL (E2E_BACKEND_URL). Used by API-only E2E specs. */
export async function loginSession(
  request: import('@playwright/test').APIRequestContext,
) {
  const creds = e2eUserCreds();
  if (!creds) return null;

  const fromSetup = readSessionFromAuthFile(creds);
  // #region agent log
  fetch('http://localhost:7327/ingest/092d89aa-ec11-4f83-9ed9-0567d2046e3c',{method:'POST',headers:{'Content-Type':'application/json','X-Debug-Session-Id':'4e07b4'},body:JSON.stringify({sessionId:'4e07b4',hypothesisId:'H4',location:'helpers.ts:loginSession',message:'loginSession start',data:{hasCreds:true,hasAuthFile:!!fromSetup,emailDomain:creds.email.split('@')[1]||''},timestamp:Date.now()})}).catch(()=>{});
  // #endregion
  if (fromSetup && (await verifyApiSession(request, fromSetup.auth))) {
    return fromSetup;
  }

  const apiBase = e2eApiBase();
  const baseURL = appBaseUrl();
  const urls = [`${baseURL}/api/players/login`, `${apiBase}/api/players/login`];
  const seen = new Set<string>();

  for (const url of urls) {
    if (seen.has(url)) continue;
    seen.add(url);
    const loginRes = await request.post(url, {
      headers: { 'Content-Type': 'application/json' },
      data: JSON.stringify({ email: creds.email, password: creds.password }),
    });
    // #region agent log
    fetch('http://localhost:7327/ingest/092d89aa-ec11-4f83-9ed9-0567d2046e3c',{method:'POST',headers:{'Content-Type':'application/json','X-Debug-Session-Id':'4e07b4'},body:JSON.stringify({sessionId:'4e07b4',hypothesisId:'H4',location:'helpers.ts:loginSession:post',message:'login POST',data:{url,status:loginRes.status(),ok:loginRes.ok()},timestamp:Date.now()})}).catch(()=>{});
    // #endregion
    if (loginRes.ok()) {
      const login = (await loginRes.json()) as {
        session_id: string;
        player: { _id?: string; id?: string };
      };
      const playerId = login.player._id || login.player.id || '';
      return { auth: bearerAuth(login.session_id), playerId, apiBase };
    }
  }
  return null;
}

function readSessionFromAuthFile(
  creds: E2ECreds,
): {
  auth: ReturnType<typeof bearerAuth>;
  playerId: string;
  apiBase: string;
} | null {
  try {
    const authPath = path.join('_build', '.auth', 'user.json');
    if (!fs.existsSync(authPath)) return null;
    const state = JSON.parse(fs.readFileSync(authPath, 'utf8')) as {
      origins?: Array<{
        localStorage?: Array<{ name: string; value: string }>;
      }>;
    };
    const storage = state.origins?.[0]?.localStorage ?? [];
    const sessionRaw = storage.find((e) => e.name === 'session_id')?.value;
    const playerRaw = storage.find((e) => e.name === 'player')?.value;
    if (!sessionRaw || !playerRaw) return null;
    const session_id = JSON.parse(sessionRaw) as string;
    const player = JSON.parse(playerRaw) as {
      _id?: string;
      id?: string;
      email?: string;
    };
    const playerEmail = typeof player.email === 'string' ? player.email : '';
    if (
      playerEmail &&
      playerEmail.toLowerCase() !== creds.email.toLowerCase()
    ) {
      return null;
    }
    const playerId = player._id || player.id || '';
    if (!session_id || !playerId) return null;
    return { auth: bearerAuth(session_id), playerId, apiBase: e2eApiBase() };
  } catch {
    return null;
  }
}

/** Verify Bearer session is still valid against /api/players/me. */
async function verifyApiSession(
  request: import('@playwright/test').APIRequestContext,
  auth: ReturnType<typeof bearerAuth>,
): Promise<boolean> {
  const baseURL = appBaseUrl();
  const apiBase = e2eApiBase();
  for (const url of [`${baseURL}/api/players/me`, `${apiBase}/api/players/me`]) {
    const me = await request.get(url, { headers: auth });
    if (me.ok()) return true;
  }
  return false;
}

/** POST /api/players/login (Caddy proxy first, then direct backend). */
export async function postLogin(
  creds: E2ECreds,
  post: (url: string) => ReturnType<Page['request']['post']>,
): Promise<LoginSession> {
  const baseURL = appBaseUrl();
  const backendURL = process.env.E2E_BACKEND_URL || 'http://127.0.0.1:50002';
  const urls = [`${baseURL}/api/players/login`, `${backendURL}/api/players/login`];
  const seen = new Set<string>();
  let lastStatus = 0;
  let lastBody = '';

  for (const url of urls) {
    if (seen.has(url)) continue;
    seen.add(url);
    const res = await post(url);
    if (res.ok()) {
      return (await res.json()) as LoginSession;
    }
    lastStatus = res.status();
    lastBody = await res.text().catch(() => '');
  }

  throw new Error(
    `Login failed for ${creds.email} (last HTTP ${lastStatus}: ${lastBody}; tried ${[...seen].join(', ')})`,
  );
}

/** Confirm Bearer session works through the frontend proxy. */
export async function verifySession(
  request: Page['request'],
  session_id: string,
): Promise<void> {
  const baseURL = appBaseUrl();
  const me = await request.get(`${baseURL}/api/players/me`, {
    headers: { Authorization: `Bearer ${session_id}` },
  });
  if (!me.ok()) {
    throw new Error(`GET /api/players/me failed: HTTP ${me.status()} ${await me.text()}`);
  }
}

/**
 * Authenticate like the SPA after login: API session + gloo_storage keys before first paint.
 */
export async function applyApiSession(page: Page, creds: E2ECreds): Promise<LoginSession> {
  // NOTE: In some Playwright versions (incl. our pre-baked Docker image), `json:` does not set
  // the expected `Content-Type: application/json` for Actix `web::Json`, leading to
  // "Invalid JSON: Content type error". Use explicit headers + JSON string body.
  const session = await postLogin(creds, (url) =>
    page.request.post(url, {
      headers: { 'Content-Type': 'application/json' },
      data: JSON.stringify({ email: creds.email, password: creds.password }),
    }),
  );
  await verifySession(page.request, session.session_id);

  await page.addInitScript(
    ({ player, session_id }) => {
      localStorage.setItem('player', JSON.stringify(player));
      localStorage.setItem('session_id', JSON.stringify(session_id));
    },
    { player: session.player, session_id: session.session_id },
  );

  await page.goto(appBaseUrl(), { waitUntil: 'domcontentloaded' });
  return session;
}

/**
 * Assert authenticated shell (nav). Do not require profile data to load — prod DB copies
 * often hit "Failed to parse profile" while the session is still valid.
 */
export async function expectLoggedIn(page: Page) {
  await expect(page).not.toHaveURL(/\/login/);
  const authedUi = page
    .getByRole('button', { name: /logout/i })
    .or(page.getByText(/^Welcome,/i))
    .or(page.getByRole('link', { name: /^profile$/i }));
  await expect(authedUi.first()).toBeVisible({ timeout: 30_000 });
}

/** Yew login inputs use oninput — Playwright fill() dispatches input. */
export async function fillYewInput(input: Locator, value: string) {
  await input.click();
  await input.fill(value);
  await input.dispatchEvent('input');
}

/** UI sign-in form (optional / slower). */
export async function loginViaSignInForm(page: Page, creds: E2ECreds) {
  await gotoApp(page, '/login');
  await waitForHeading(page, /sign in to your account/i);

  const emailInput = page.locator('main input[name="email"], main input[type="email"]').first();
  const passwordInput = page.locator('main input[name="password"], main input[type="password"]').first();
  await fillYewInput(emailInput, creds.email);
  await fillYewInput(passwordInput, creds.password);

  await page.locator('main form button[type="submit"]').first().click();

  await expect(page.getByRole('heading', { name: /sign in to your account/i })).not.toBeVisible({
    timeout: 45_000,
  });
  await expectLoggedIn(page);
}

/** API login + profile page (gate default). */
export async function login(page: Page, creds: E2ECreds) {
  await applyApiSession(page, creds);
  await expectLoggedIn(page);
}
