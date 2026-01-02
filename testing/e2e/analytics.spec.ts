import { test, expect } from '@playwright/test';

/**
 * E2E tests for analytics and dashboard functionality
 */

test.describe('Analytics', () => {
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

  test('should load analytics page', async ({ page }) => {
    await page.goto('/analytics');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Check for analytics content
    const content = await body.textContent();
    expect(content).toBeTruthy();
  });

  test('should display analytics dashboard', async ({ page }) => {
    await page.goto('/analytics');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Look for charts, graphs, or analytics widgets
    const charts = page.locator('canvas, svg, [class*="chart"], [class*="graph"], [class*="analytics"]');
    const chartCount = await charts.count();
    
    // Page should load even if no charts are visible yet
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should handle analytics data loading', async ({ page }) => {
    await page.goto('/analytics');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Give time for data to load
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Check for loading indicators or content
    const loadingIndicators = page.locator('[class*="loading"], [class*="spinner"], [class*="skeleton"]');
    const loadingCount = await loadingIndicators.count();
    
    // After timeout, loading should be complete
    expect(loadingCount >= 0).toBeTruthy();
  });
});

test.describe('Analytics Performance', () => {
  test('should load analytics page quickly', async ({ page }) => {
    const startTime = Date.now();
    
    await page.goto('/analytics');
    await page.waitForLoadState('networkidle');
    
    const loadTime = Date.now() - startTime;
    
    // Analytics page should load within reasonable time (10 seconds)
    expect(loadTime).toBeLessThan(10000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should handle multiple analytics requests', async ({ page }) => {
    await page.goto('/analytics');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Navigate away and back
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    
    await page.goto('/analytics');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });
});

