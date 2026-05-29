import type { FullConfig } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

/**
 * Log in via POST /api/players/login and write Playwright storageState.
 * Tries frontend origin first (Caddy /api proxy), then direct backend URL.
 */
export default async function globalSetup(config: FullConfig) {
  const email = process.env.E2E_USER_EMAIL;
  const password = process.env.E2E_USER_PASSWORD;
  if (!email || !password) return;

  const baseURL =
    process.env.PLAYWRIGHT_BASE_URL ||
    (typeof config.projects[0]?.use?.baseURL === 'string' ? config.projects[0].use.baseURL : undefined) ||
    'http://127.0.0.1:50003';
  const backendURL = process.env.E2E_BACKEND_URL || 'http://127.0.0.1:50002';

  const urls = [`${baseURL}/api/players/login`, `${backendURL}/api/players/login`];
  const seen = new Set<string>();
  let body: { player: Record<string, unknown>; session_id: string } | null = null;
  let lastStatus = 0;
  let lastText = '';

  for (const url of urls) {
    if (seen.has(url)) continue;
    seen.add(url);
    const loginRes = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password }),
    });
    if (loginRes.ok) {
      body = (await loginRes.json()) as typeof body;
      break;
    }
    lastStatus = loginRes.status;
    lastText = await loginRes.text().catch(() => '');
  }

  if (!body) {
    throw new Error(
      `E2E globalSetup: login failed HTTP ${lastStatus}: ${lastText} (tried ${[...seen].join(', ')})`,
    );
  }

  const { player, session_id } = body;
  const origin = new URL(baseURL).origin;
  const authDir = path.join('_build', '.auth');
  fs.mkdirSync(authDir, { recursive: true });

  const storageState = {
    cookies: [] as Array<Record<string, unknown>>,
    origins: [
      {
        origin,
        localStorage: [
          { name: 'player', value: JSON.stringify(player) },
          { name: 'session_id', value: JSON.stringify(session_id) },
        ],
      },
    ],
  };

  fs.writeFileSync(path.join(authDir, 'user.json'), JSON.stringify(storageState, null, 2));
}
