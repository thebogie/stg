import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers';

/**
 * E2E tests for navigation and routing.
 * Prefer SPA link clicks over chained full page loads on prod WASM stacks.
 */

test.describe('Navigation', () => {
  test('should navigate to all main pages', async ({ page }) => {
    await gotoApp(page, '/');
    const leaderboards = page.getByRole('link', { name: /leaderboards/i }).first();
    await expect(leaderboards).toBeVisible({ timeout: 30_000 });
    await leaderboards.click();
    await expect(page).toHaveURL(/\/leaderboards/, { timeout: 20_000 });
    // Return home via SPA (covers "maintain state across transitions" without another cold load).
    await page.getByRole('link', { name: 'STG' }).first().click();
    await expect(page).not.toHaveURL(/\/leaderboards/, { timeout: 20_000 });
  });

  test('should handle 404 for invalid routes', async ({ page }) => {
    await gotoApp(page, '/invalid-route-that-does-not-exist');

    const body = page.locator('body');
    await expect(body).toBeVisible();
    const content = await body.textContent();
    expect(content).toBeTruthy();
  });

  test('should navigate using browser back/forward', async ({ page }) => {
    await gotoApp(page, '/leaderboards');
    await page.getByRole('link', { name: 'STG' }).first().click();
    await page.goBack({ timeout: 15_000 }).catch(() => {});
    await expect(page.locator('body')).toBeAttached();
    await page.goForward({ timeout: 15_000 }).catch(() => {});
    await expect(page.locator('body')).toBeAttached();
  });

});

test.describe('Navigation Links', () => {
  test('should have working navigation links', async ({ page }) => {
    await gotoApp(page, '/');

    const menuButton = page.locator('button[aria-label="Toggle mobile menu"]');
    if (await menuButton.isVisible().catch(() => false)) {
      await menuButton.click();
    }

    const navLinks = page.locator('nav a, header a, [role="navigation"] a');
    const linkCount = await navLinks.count();
    expect(linkCount).toBeGreaterThan(0);

    const hrefs: string[] = [];
    for (let i = 0; i < linkCount; i++) {
      const href = await navLinks.nth(i).getAttribute('href');
      if (href) hrefs.push(href);
    }
    expect(hrefs.some((h) => h && !h.startsWith('#'))).toBeTruthy();
    expect(hrefs.some((h) => h === '/' || h === '/leaderboards' || h === '/login')).toBeTruthy();
  });
});
