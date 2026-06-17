import { test, expect } from '@playwright/test';
import { appBaseUrl, gotoApp } from './helpers';

/** CI E2E uses PLAYWRIGHT_BASE_URL; standalone worker smoke uses STG_BASE_URL. */
function smokeBaseUrl() {
  return process.env.STG_BASE_URL || appBaseUrl();
}

test.describe('smoke.stg', () => {
  test('home page loads', async ({ page }) => {
    // #region agent log
    fetch('http://localhost:7327/ingest/092d89aa-ec11-4f83-9ed9-0567d2046e3c',{method:'POST',headers:{'Content-Type':'application/json','X-Debug-Session-Id':'62fa5a'},body:JSON.stringify({sessionId:'62fa5a',location:'smoke.stg.spec.ts:home',message:'smoke base url',data:{base:smokeBaseUrl(),playwright:process.env.PLAYWRIGHT_BASE_URL,stg:process.env.STG_BASE_URL},timestamp:Date.now(),hypothesisId:'H1'})}).catch(()=>{});
    // #endregion
    await gotoApp(page, '/');
  });

  test('version API responds', async ({ request }) => {
    const base = smokeBaseUrl();
    const res = await request.get(`${base}/api/version`);
    expect(res.ok()).toBeTruthy();
  });
});
