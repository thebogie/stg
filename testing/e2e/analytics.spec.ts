import { test, expect } from '@playwright/test';
import { gotoApp, e2eUserCreds, e2eApiBase, loginSession } from './helpers';

/**
 * E2E tests for analytics and dashboard functionality
 * (authenticated via global-setup storageState — chromium-authenticated project)
 */

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

test.describe('Analytics tabs', () => {
  test('each tab loads and requests tab API with timezone', async ({ page }) => {
    const tabRequests: string[] = [];
    page.on('request', (req) => {
      const url = req.url();
      if (url.includes('/api/analytics/tabs/') && url.includes('timezone=')) {
        tabRequests.push(url);
      }
    });

    await gotoApp(page, '/analytics');
    await expect(page.getByRole('heading', { name: /analytics statistics/i })).toBeVisible({
      timeout: 30_000,
    });

    for (const tabName of ['Overview', 'Contests', 'Venues', 'Games', 'Players']) {
      await page.getByRole('button', { name: new RegExp(`^${tabName}$`, 'i') }).click();
      await page.waitForTimeout(500);
    }

    expect(tabRequests.some((u) => u.includes('/tabs/overview'))).toBeTruthy();
    expect(tabRequests.some((u) => u.includes('/tabs/contests'))).toBeTruthy();
    expect(tabRequests.some((u) => u.includes('/tabs/venues'))).toBeTruthy();
    expect(tabRequests.some((u) => u.includes('/tabs/games'))).toBeTruthy();
    expect(tabRequests.some((u) => u.includes('timezone='))).toBeTruthy();
  });

  test('overview shows week-over-week section', async ({ page }) => {
    await gotoApp(page, '/analytics');
    await expect(page.getByRole('heading', { name: /week-over-week growth/i })).toBeVisible({
      timeout: 45_000,
    });
  });

  test('contests tab shows recent contests section', async ({ page }) => {
    await gotoApp(page, '/analytics');
    await page.getByRole('button', { name: /^contests$/i }).click();
    await expect(page.getByRole('heading', { name: /recent contests/i })).toBeVisible({
      timeout: 45_000,
    });
    // Table headers only render when contest rows exist; empty DB shows no-data copy instead.
    await expect(
      page
        .getByRole('columnheader', { name: /^contest$/i })
        .or(page.getByText(/no recent contests found/i))
        .first(),
    ).toBeVisible({ timeout: 10_000 });
  });
});

test.describe('Analytics API tabs', () => {
  test('public tab endpoints accept timezone query', async ({ request }) => {
    const apiBase = e2eApiBase();
    const paths = [
      '/api/analytics/tabs/overview?timezone=America/Chicago',
      '/api/analytics/tabs/contests?timezone=UTC',
      '/api/analytics/tabs/venues?timezone=UTC',
      '/api/analytics/tabs/games?timezone=UTC',
    ];
    for (const path of paths) {
      const res = await request.get(`${apiBase}${path}`);
      const text = await res.text();
      expect(res.ok(), `${path}: ${res.status()} ${text}`).toBeTruthy();
      const body = JSON.parse(text) as { timezone?: string };
      expect(body.timezone).toBeTruthy();
    }
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
  test('should load analytics page within CI budget', async ({ page }) => {
    const startTime = Date.now();
    await gotoApp(page, '/analytics');
    await expect(page.getByRole('heading', { name: /analytics statistics/i })).toBeVisible({
      timeout: 45_000,
    });
    const loadTime = Date.now() - startTime;
    const maxMs = process.env.CI ? 60_000 : 15_000;
    expect(loadTime).toBeLessThan(maxMs);
  });

  test('should handle multiple analytics navigations', async ({ page }) => {
    test.setTimeout(process.env.CI ? 180_000 : 90_000);

    await gotoApp(page, '/analytics');
    await expect(page.getByRole('heading', { name: /analytics statistics/i })).toBeVisible({
      timeout: 45_000,
    });

    // SPA route changes avoid extra cold WASM reloads (each gotoApp can take ~30s in CI).
    await page.getByRole('link', { name: /leaderboards/i }).click();
    await expect(page).toHaveURL(/\/leaderboards/);

    await page.getByRole('link', { name: /^statistics$/i }).click();
    await expect(page.getByRole('heading', { name: /analytics statistics/i })).toBeVisible({
      timeout: 45_000,
    });
  });
});
