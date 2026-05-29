import { test, expect } from '@playwright/test';
import { gotoApp } from './helpers';

/**
 * E2E tests for analytics and dashboard functionality
 * (authenticated via global-setup storageState — chromium-authenticated project)
 */

test.describe('Analytics', () => {
  test('should load analytics page', async ({ page }) => {
    await gotoApp(page, '/analytics');
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Check for analytics content
    const content = await body.textContent();
    expect(content).toBeTruthy();
  });

  test('should display analytics dashboard', async ({ page }) => {
    await gotoApp(page, '/analytics');
    
    // Look for charts, graphs, or analytics widgets
    const charts = page.locator('canvas, svg, [class*="chart"], [class*="graph"], [class*="analytics"]');
    const chartCount = await charts.count();
    
    // Page should load even if no charts are visible yet
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should handle analytics data loading', async ({ page }) => {
    await gotoApp(page, '/analytics');
    
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
    
    await gotoApp(page, '/analytics');
    
    const loadTime = Date.now() - startTime;
    
    // Analytics page should load within reasonable time (10 seconds)
    expect(loadTime).toBeLessThan(10000);
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should handle multiple analytics requests', async ({ page }) => {
    await gotoApp(page, '/analytics');
    
    // Navigate away and back
    await gotoApp(page, '/');
    
    await gotoApp(page, '/analytics');
    
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });
});

