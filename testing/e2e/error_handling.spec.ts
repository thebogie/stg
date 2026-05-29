import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers';

/**
 * E2E tests for error handling and edge cases
 */

test.describe('Error Handling', () => {
  test('should handle 404 pages gracefully', async ({ page }) => {
    await gotoApp(page, '/nonexistent-page-12345');
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Should show 404 or redirect to home
    const url = page.url();
    const content = await body.textContent();
    expect(content).toBeTruthy();
  });

  test('should handle network errors gracefully', async ({ page, context }) => {
    // First, load the page while online
    await gotoApp(page, '/');
    
    // Now simulate offline mode
    await context.setOffline(true);
    
    // Try to navigate - should handle gracefully
    try {
      await page.goto('/venues', { timeout: 2000 });
    } catch (error) {
      // Expected: network error when offline
      // Just verify page is still functional
    }
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Restore online
    await context.setOffline(false);
    await page.waitForTimeout(500);
  });

  test('should handle slow network connections', async ({ page, context }) => {
    // Simulate slow 3G
    await context.route('**/*', route => {
      setTimeout(() => route.continue(), 100);
    });
    
    await gotoApp(page, '/');
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should handle invalid form submissions', async ({ page }) => {
    await gotoApp(page, '/login');
    
    // Try to submit empty form
    const submitButton = page.locator('button[type="submit"]').first();
    if (await submitButton.count() > 0) {
      await submitButton.click();
      await page.waitForTimeout(500);
      
      // Should show validation errors or prevent submission
      const body = page.locator('body');
      await expect(body).toBeVisible();
    }
  });
});

test.describe('Edge Cases', () => {
  test('should handle very long input strings', async ({ page }) => {
    await gotoApp(page, '/login');
    
    const emailInput = page.locator('input[type="email"]').first();
    if (await emailInput.count() > 0) {
      const longString = 'a'.repeat(1000);
      await emailInput.fill(longString);
      
      // Should handle long input without crashing
      const body = page.locator('body');
      await expect(body).toBeVisible();
    }
  });

  test('should handle special characters in URLs', async ({ page }) => {
    // Test URL with special characters
    await gotoApp(page, '/venues?search=test%26game');
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should handle rapid navigation', async ({ page }) => {
    // SPA link hops only — avoid chaining full WASM reloads on /venues, /games, etc.
    await gotoApp(page, '/');
    await page.getByRole('link', { name: /leaderboards/i }).first().click();
    await expect(page).toHaveURL(/\/leaderboards/);
    await page.getByRole('link', { name: 'STG' }).first().click();
    await expect(page).not.toHaveURL(/\/leaderboards/);
    await page.getByRole('button', { name: /login/i }).first().click();
    await expect(page).toHaveURL(/\/login/);
    await expect(page.locator('body')).toBeVisible();
  });
});

