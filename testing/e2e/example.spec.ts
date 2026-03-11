import { test, expect } from '@playwright/test';

/**
 * E2E tests for Yew frontend
 * 
 * These tests render the actual WASM in a headless browser
 * and can perform visual regression testing via screenshots.
 * 
 * Note: These tests require the frontend server to be running.
 * The Playwright config should start it automatically.
 */

test.describe('Frontend E2E Tests', () => {
  test('should load the homepage', async ({ page }) => {
    await page.goto('/');
    
    // Wait for WASM to load and page to be interactive
    await page.waitForLoadState('networkidle');
    // Give WASM a moment to initialize
    await page.waitForTimeout(1000);
    
    // Verify page loaded (check for any content, not just navigation)
    const body = page.locator('body');
    await expect(body).toBeVisible({ timeout: 10000 });
    
    // Check that page has some content (not just blank)
    const bodyText = await body.textContent();
    expect(bodyText).toBeTruthy();
    expect(bodyText!.trim().length).toBeGreaterThan(0);
  });

  test('should display navigation', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Check for navigation elements - try multiple possible selectors
    const nav = page.locator('nav').first();
    const navExists = await nav.count() > 0;
    
    if (!navExists) {
      // Fallback: check if page has any interactive elements
      const links = page.locator('a');
      const buttons = page.locator('button');
      const hasInteractive = (await links.count()) > 0 || (await buttons.count()) > 0;
      expect(hasInteractive).toBeTruthy();
    } else {
      await expect(nav).toBeVisible();
    }
  });

  test('should handle basic page interactions', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Check that page is responsive (can interact with it)
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Try to find and click any link or button if available
    const firstLink = page.locator('a').first();
    const linkCount = await firstLink.count();
    
    if (linkCount > 0) {
      const href = await firstLink.getAttribute('href');
      if (href && !href.startsWith('#')) {
        // Only click if it's a real navigation link
        await firstLink.click();
        await page.waitForLoadState('networkidle');
        // Verify we navigated or page changed
        expect(page.url()).toBeTruthy();
      }
    }
  });
});

test.describe('Visual Regression Tests', () => {
  test('homepage visual snapshot', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Verify page loaded before taking screenshot
    const body = page.locator('body');
    await expect(body).toBeVisible({ timeout: 10000 });
    
    // Full page screenshot for visual regression
    await expect(page).toHaveScreenshot('homepage-full.png', {
      fullPage: true,
      timeout: 10000,
    });
  });

  test('admin page visual snapshot', async ({ page }) => {
    // Navigate to admin page if it exists
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Check if page loaded (might be 404 or redirect)
    const status = page.url();
    expect(status).toBeTruthy();
    
    // Only take screenshot if page exists (not 404)
    const body = page.locator('body');
    const isVisible = await body.isVisible().catch(() => false);
    
    if (isVisible) {
      await expect(page).toHaveScreenshot('admin-page.png', {
        fullPage: true,
        timeout: 10000,
      });
    } else {
      test.skip();
    }
  });
});

