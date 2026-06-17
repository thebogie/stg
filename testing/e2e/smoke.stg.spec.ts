import { test, expect } from '@playwright/test';
import { appBaseUrl, gotoApp } from './helpers';

/** CI E2E uses PLAYWRIGHT_BASE_URL; standalone worker smoke uses STG_BASE_URL. */
function smokeBaseUrl() {
  return process.env.STG_BASE_URL || appBaseUrl();
}

test.describe('smoke.stg', () => {
  test('home page loads', async ({ page }) => {
    await gotoApp(page, '/');
  });

  test('version API responds', async ({ request }) => {
    const base = smokeBaseUrl();
    const res = await request.get(`${base}/api/version`);
    expect(res.ok()).toBeTruthy();
  });
});
