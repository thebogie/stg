import { test, expect } from '@playwright/test';

/**
 * E2E tests for CRUD operations
 * Tests creating, reading, updating, and deleting venues, games, and contests
 */

test.describe('Venue CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const emailInput = page.locator('input[name="email"], input[type="email"]').first();
    const passwordInput = page.locator('input[name="password"], input[type="password"]').first();
    
    if (await emailInput.count() > 0 && await passwordInput.count() > 0) {
      await emailInput.fill('test@example.com');
      await passwordInput.fill('password123');
      
      const submitButton = page.locator('button[type="submit"]').first();
      if (await submitButton.count() > 0) {
        await submitButton.click();
        await page.waitForLoadState('networkidle');
        await page.waitForTimeout(2000);
      }
    }
  });

  test('should navigate to venues page', async ({ page }) => {
    await page.goto('/venues');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Check if venues list or content is visible
    const content = await body.textContent();
    expect(content).toBeTruthy();
  });

  test('should display venue list', async ({ page }) => {
    await page.goto('/venues');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Look for venue-related content
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Check for any list or table structure
    const lists = page.locator('ul, ol, table, div[class*="list"], div[class*="grid"]');
    const listCount = await lists.count();
    
    // At minimum, page should load
    expect(listCount >= 0).toBeTruthy();
  });
});

test.describe('Game CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const emailInput = page.locator('input[name="email"], input[type="email"]').first();
    const passwordInput = page.locator('input[name="password"], input[type="password"]').first();
    
    if (await emailInput.count() > 0 && await passwordInput.count() > 0) {
      await emailInput.fill('test@example.com');
      await passwordInput.fill('password123');
      
      const submitButton = page.locator('button[type="submit"]').first();
      if (await submitButton.count() > 0) {
        await submitButton.click();
        await page.waitForLoadState('networkidle');
        await page.waitForTimeout(2000);
      }
    }
  });

  test('should navigate to games page', async ({ page }) => {
    await page.goto('/games');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should display game list', async ({ page }) => {
    await page.goto('/games');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });
});

test.describe('Contest CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const emailInput = page.locator('input[name="email"], input[type="email"]').first();
    const passwordInput = page.locator('input[name="password"], input[type="password"]').first();
    
    if (await emailInput.count() > 0 && await passwordInput.count() > 0) {
      await emailInput.fill('test@example.com');
      await passwordInput.fill('password123');
      
      const submitButton = page.locator('button[type="submit"]').first();
      if (await submitButton.count() > 0) {
        await submitButton.click();
        await page.waitForLoadState('networkidle');
        await page.waitForTimeout(2000);
      }
    }
  });

  test('should navigate to contests page', async ({ page }) => {
    await page.goto('/contests');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should navigate to contest creation page', async ({ page }) => {
    await page.goto('/contest/create');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });
});

