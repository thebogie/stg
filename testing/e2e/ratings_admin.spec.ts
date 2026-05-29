import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers';

test.describe('Ratings / Admin (smoke)', () => {
  test('leaderboards page exposes SkillRating category', async ({ page }) => {
    await gotoApp(page, '/leaderboards');
    await expect(page.getByRole('heading', { name: /leaderboards/i })).toBeVisible();

    const category = page.locator('select').first();
    await expect(category).toBeVisible();

    // Should contain SkillRating option (Glicko2-backed leaderboard category).
    const optionsText = await category.locator('option').allTextContents();
    expect(optionsText.join(' ')).toMatch(/skill\s*rating/i);
  });

  test('admin page is protected for non-admin users', async ({ page }) => {
    await gotoApp(page, '/admin');
    await expect(page.locator('body')).toBeVisible();

    // Either show access denied, or redirect to login; both mean "protected".
    const accessDenied = page.getByRole('heading', { name: /access denied/i }).first();
    const loginHeading = page.getByRole('heading', { name: /sign in|login/i }).first();

    const ok =
      (await accessDenied.isVisible().catch(() => false)) ||
      (await loginHeading.isVisible().catch(() => false)) ||
      page.url().includes('/login');

    expect(ok).toBeTruthy();
  });
});

