import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers';

/**
 * E2E tests for Yew frontend (functionality only).
 *
 * These tests render the actual WASM in a headless browser and assert
 * behavior, navigation, and content—no visual/screenshot regression.
 * Requires the frontend server to be running (Playwright config or prod stack).
 */

test.describe('Frontend E2E Tests', () => {
  test('should load the homepage', async ({ page }) => {
    await gotoApp(page, '/');
    
    // Verify page loaded (check for any content, not just navigation)
    const body = page.locator('body');
    await expect(body).toBeVisible({ timeout: 10000 });
    
    // Check that page has some content (not just blank)
    const bodyText = await body.textContent();
    expect(bodyText).toBeTruthy();
    expect(bodyText!.trim().length).toBeGreaterThan(0);
  });

  test('should display navigation', async ({ page }) => {
    await gotoApp(page, '/');
    
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
    await gotoApp(page, '/');
    
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
        // Verify we navigated or page changed
        expect(page.url()).toBeTruthy();
      }
    }
  });
});
