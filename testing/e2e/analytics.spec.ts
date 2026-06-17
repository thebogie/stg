import { test, expect } from '@playwright/test';
import { gotoApp, e2eUserCreds, e2eApiBase, bearerAuth } from './helpers';

/**
 * E2E tests for analytics and dashboard functionality
 * (authenticated via global-setup storageState — chromium-authenticated project)
 */

async function loginSession(request: import('@playwright/test').APIRequestContext) {
  const creds = e2eUserCreds();
  if (!creds) return null;
  const apiBase = e2eApiBase();
  const loginRes = await request.post(`${apiBase}/api/players/login`, {
    headers: { 'Content-Type': 'application/json' },
    data: JSON.stringify({ email: creds.email, password: creds.password }),
  });
  if (!loginRes.ok()) return null;
  const login = (await loginRes.json()) as {
    session_id: string;
    player: { _id?: string; id?: string };
  };
  const playerId =
    login.player._id || login.player.id || '';
  return { auth: bearerAuth(login.session_id), playerId, apiBase };
}

test.describe('Analytics', () => {
  test('should load analytics page', async ({ page }) => {
    await gotoApp(page, '/analytics');

    const body = page.locator('body');
    await expect(body).toBeVisible();

    const content = await body.textContent();
    expect(content).toBeTruthy();
  });

  test('should display analytics dashboard', async ({ page }) => {
    await gotoApp(page, '/analytics');

    await expect(page.getByRole('heading', { name: /analytics statistics/i })).toBeVisible({
      timeout: 30_000,
    });
  });

  test('should handle analytics data loading', async ({ page }) => {
    await gotoApp(page, '/analytics');

    await expect(page.getByRole('heading', { name: /analytics statistics/i })).toBeVisible({
      timeout: 30_000,
    });

    // Overview tab should show platform stat cards after load
    await expect(page.getByText(/total players/i).first()).toBeVisible({ timeout: 45_000 });
  });
});

test.describe('Analytics API data', () => {
  test.skip(
    !e2eUserCreds(),
    'Set E2E_USER_EMAIL and E2E_USER_PASSWORD (see testing/e2e/global-setup.ts)',
  );

  test('chart endpoints return series data', async ({ request }) => {
    const session = await loginSession(request);
    expect(session, 'login required for analytics API').toBeTruthy();
    const { apiBase } = session!;

    const charts = [
      '/api/analytics/charts/player-performance-distribution',
      '/api/analytics/charts/contest-completion-by-game',
      '/api/analytics/charts/player-retention-cohort',
    ];

    for (const path of charts) {
      const res = await request.get(`${apiBase}${path}`);
      const text = await res.text();
      expect(res.ok(), `${path}: HTTP ${res.status()} ${text}`).toBeTruthy();
      const body = JSON.parse(text) as {
        data?: { SingleSeries?: unknown[]; MultiSeries?: unknown };
      };
      const series = body.data?.SingleSeries;
      expect(
        Array.isArray(series),
        `${path} should return SingleSeries array`,
      ).toBeTruthy();
    }
  });

  test('communities and networking endpoints return structured JSON', async ({ request }) => {
    const session = await loginSession(request);
    expect(session, 'login required').toBeTruthy();
    const { auth, playerId, apiBase } = session!;
    expect(playerId.length).toBeGreaterThan(0);

    const encoded = encodeURIComponent(playerId);
    const communitiesRes = await request.get(
      `${apiBase}/api/analytics-enhanced/communities/${encoded}?min_contests=2`,
      { headers: auth },
    );
    const communitiesText = await communitiesRes.text();
    expect(
      communitiesRes.ok(),
      `communities: HTTP ${communitiesRes.status()} ${communitiesText}`,
    ).toBeTruthy();
    const communities = JSON.parse(communitiesText) as {
      gaming_communities?: unknown[];
    };
    expect(Array.isArray(communities.gaming_communities)).toBeTruthy();

    const networkingRes = await request.get(
      `${apiBase}/api/analytics-enhanced/networking/${encoded}`,
      { headers: auth },
    );
    const networkingText = await networkingRes.text();
    expect(
      networkingRes.ok(),
      `networking: HTTP ${networkingRes.status()} ${networkingText}`,
    ).toBeTruthy();
    const networking = JSON.parse(networkingText) as {
      opponent_analysis?: unknown[];
      network_metrics?: { total_opponents?: number };
    };
    expect(Array.isArray(networking.opponent_analysis)).toBeTruthy();
    expect(typeof networking.network_metrics?.total_opponents).toBe('number');
  });
});

test.describe('Analytics Players tab', () => {
  test('should show gaming communities and social network sections', async ({ page }) => {
    await gotoApp(page, '/analytics');

    await page.getByRole('button', { name: /^players$/i }).click();

    await expect(page.getByRole('heading', { name: /gaming communities/i })).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByRole('heading', { name: /social network/i })).toBeVisible({
      timeout: 30_000,
    });
  });
});

test.describe('Analytics Performance', () => {
  test('should load analytics page quickly', async ({ page }) => {
    const startTime = Date.now();

    await gotoApp(page, '/analytics');

    const loadTime = Date.now() - startTime;

    expect(loadTime).toBeLessThan(10000);

    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should handle multiple analytics requests', async ({ page }) => {
    await gotoApp(page, '/analytics');

    await gotoApp(page, '/');

    await gotoApp(page, '/analytics');

    const body = page.locator('body');
    await expect(body).toBeVisible();
  });
});
