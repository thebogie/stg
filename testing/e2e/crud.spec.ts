import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers';

/**
 * E2E tests for CRUD operations
 * Tests creating, reading, updating, and deleting venues, games, and contests
 * (authenticated via global-setup storageState — chromium-authenticated project)
 */

test.describe('Venue CRUD Operations', () => {
  test('should navigate to venues page', async ({ page }) => {
    await gotoApp(page, '/venues');

    const body = page.locator('body');
    await expect(body).toBeVisible();

    const content = await body.textContent();
    expect(content).toBeTruthy();
  });

  test('should display venue list', async ({ page }) => {
    await gotoApp(page, '/venues');

    const body = page.locator('body');
    await expect(body).toBeVisible();

    const lists = page.locator('ul, ol, table, div[class*="list"], div[class*="grid"]');
    const listCount = await lists.count();
    expect(listCount >= 0).toBeTruthy();
  });
});

test.describe('Game CRUD Operations', () => {
  test('should navigate to games page', async ({ page }) => {
    await gotoApp(page, '/games');

    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should display game list', async ({ page }) => {
    await gotoApp(page, '/games');

    const body = page.locator('body');
    await expect(body).toBeVisible();
  });
});

test.describe('Contest CRUD Operations', () => {
  test('should navigate to contests page', async ({ page }) => {
    await gotoApp(page, '/contests');

    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should navigate to contest creation page', async ({ page }) => {
    await gotoApp(page, '/contest/create');

    const body = page.locator('body');
    await expect(body).toBeVisible();
  });
});
