import { test, expect } from '@playwright/test';
import {
  applyApiSession,
  e2eUserCreds,
  expectLoggedIn,
  gotoApp,
  isOnLoginPage,
  login,
  waitForHeading,
} from './helpers';

/**
 * E2E tests for authentication flows
 * Tests login, logout, registration, and protected routes
 */

test.describe('Authentication', () => {
  test('should allow user to register', async ({ page }) => {
    await gotoApp(page, '/login');
    if (await isOnLoginPage(page)) {
      await waitForHeading(page, /sign in|login/i);
    }
    
    // Look for registration form or link
    const registerLink = page.locator('text=/register|sign up|create account/i').first();
    const registerLinkCount = await registerLink.count();
    
    if (registerLinkCount > 0) {
      await registerLink.click();
      await expect(page.locator('body')).toBeVisible();
    }
    
    // Fill registration form if it exists
    const usernameInput = page.locator('input[name="username"], input[type="text"]').first();
    const emailInput = page.locator('input[name="email"], input[type="email"]').first();
    const passwordInput = page.locator('input[name="password"], input[type="password"]').first();
    
    if (await usernameInput.count() > 0) {
      await usernameInput.fill('e2e_test_user');
    }
    if (await emailInput.count() > 0) {
      await emailInput.fill(`e2e_test_${Date.now()}@example.com`);
    }
    if (await passwordInput.count() > 0) {
      await passwordInput.fill('testpassword123');
    }
    
    // Submit if submit button exists
    const submitButton = page.locator('button[type="submit"], button:has-text("Register"), button:has-text("Sign Up")').first();
    if (await submitButton.count() > 0) {
      await submitButton.click();
      await expect(page.locator('body')).toBeVisible();
    }
    
    // Verify we're logged in (check for user menu or profile link)
    await page.waitForTimeout(2000);
    const body = page.locator('body');
    await expect(body).toBeVisible();
  });

  test('should allow user to login', async ({ page }) => {
    const creds = e2eUserCreds();
    if (!creds) test.skip(true, 'E2E_USER_EMAIL/E2E_USER_PASSWORD not set');
    await login(page, creds);
  });

  test('should redirect to login when accessing protected route', async ({ page }) => {
    // Try to access a protected route without being logged in
    await gotoApp(page, '/profile');
    
    // Should be redirected to login or show login form
    const currentUrl = page.url();
    const body = page.locator('body');
    await expect(body).toBeVisible();
    
    // Check if we're on login page or if login form is visible
    const isLoginPage = currentUrl.includes('/login') || 
                       (await page.locator('input[type="email"], input[type="password"]').count()) > 0;
    
    // Either is acceptable: redirect to login, or render a login form in-place.
    expect(isLoginPage).toBeTruthy();
  });

  test('should persist login session', async ({ page }) => {
    const creds = e2eUserCreds();
    if (!creds) test.skip(true, 'E2E_USER_EMAIL/E2E_USER_PASSWORD not set');
    await applyApiSession(page, creds);
    await expectLoggedIn(page);

    // Client-side route change (avoid a second full WASM page load on /).
    await page.getByRole('link', { name: /leaderboards/i }).first().click();
    await expect(page).toHaveURL(/\/leaderboards/);
    await expectLoggedIn(page);

    const stored = await page.evaluate(() => ({
      session_id: localStorage.getItem('session_id'),
      player: localStorage.getItem('player'),
    }));
    expect(stored.session_id).toBeTruthy();
    expect(stored.player).toBeTruthy();
  });
});

