import { test, expect, type Page } from '@playwright/test';
import {
  e2eAdminCreds,
  e2eApiBase,
  fillYewInput,
  gotoApp,
  loginAdmin,
  storedSessionId,
} from './helpers';

test.describe.configure({ mode: 'serial' });

test.describe('Admin Users tab', () => {
  test.beforeEach(async ({ page }) => {
    if (!e2eAdminCreds()) {
      test.skip(true, 'E2E_ADMIN_EMAIL/E2E_ADMIN_PASSWORD not set');
    }
    await loginAdmin(page);
  });

  async function openUsersTab(page: Page) {
    await gotoApp(page, '/admin');
    await expect(page.getByRole('heading', { name: /administrator dashboard/i })).toBeVisible({
      timeout: 30_000,
    });
    await page.getByRole('button', { name: /users/i }).click();
    await expect(page.getByRole('heading', { name: /user management/i })).toBeVisible();
    await expect(page.getByRole('button', { name: '+ Add player' })).toBeVisible();
  }

  async function searchUser(page: Page, query: string) {
    const search = page.getByPlaceholder('Search by handle or email');
    await fillYewInput(search, query);
    await page.getByRole('button', { name: 'Search' }).click();
  }

  test('renders Users tab for admin', async ({ page }) => {
    await openUsersTab(page);
    await expect(page.getByPlaceholder('Search by handle or email')).toBeVisible();
  });

  test('creates a player via UI and finds them in search', async ({ page }) => {
    const ts = Date.now();
    const email = `e2e_admin_ui_${ts}@example.test`;
    const handle = `e2e_ui_${ts}`;

    await openUsersTab(page);
    await page.getByRole('button', { name: '+ Add player' }).click();
    await expect(page.getByRole('heading', { name: 'Add player' })).toBeVisible();

    const panel = page.locator('.users-section').filter({ hasText: 'Add player' });
    const inputs = panel.locator('input:not([type="checkbox"])');
    await fillYewInput(inputs.nth(0), 'E2E');
    await fillYewInput(inputs.nth(1), handle);
    await fillYewInput(inputs.nth(2), email);
    await fillYewInput(inputs.nth(3), 'password123');

    await page.getByRole('button', { name: 'Create player' }).click();
    await expect(page.getByText('Player created', { exact: true })).toBeVisible({
      timeout: 20_000,
    });

    await searchUser(page, handle);
    const row = page.locator('tr', { hasText: handle });
    await expect(row).toBeVisible({ timeout: 20_000 });
    await expect(row.getByText(email)).toBeVisible();
  });

  test('deactivates and reactivates a player from the edit panel', async ({ page }) => {
    const ts = Date.now();
    const email = `e2e_admin_deact_${ts}@example.test`;
    const handle = `e2e_deact_${ts}`;
    const password = 'password123';

    const sessionId = await storedSessionId(page);
    const createRes = await page.request.post(`${e2eApiBase()}/api/admin/users`, {
      headers: {
        Authorization: `Bearer ${sessionId}`,
        'Content-Type': 'application/json',
      },
      data: {
        firstname: 'Deactivate',
        handle,
        email,
        password,
        is_admin: false,
      },
    });
    expect(createRes.status(), await createRes.text()).toBe(201);

    await openUsersTab(page);
    await searchUser(page, email);
    const row = page.locator('tr', { hasText: email });
    await expect(row).toBeVisible({ timeout: 20_000 });
    await row.getByRole('button', { name: 'Edit' }).click();

    await expect(page.getByRole('heading', { name: new RegExp(`Edit: ${handle}`) })).toBeVisible();

    page.once('dialog', (dialog) => dialog.accept());
    await page.getByRole('button', { name: 'Deactivate account' }).click();
    await expect(page.getByText('Player deactivated', { exact: true })).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByRole('button', { name: 'Reactivate account' })).toBeVisible();

    const loginBlocked = await page.request.post(`${e2eApiBase()}/api/players/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: { email, password },
    });
    expect(loginBlocked.status()).toBe(403);

    page.once('dialog', (dialog) => dialog.accept());
    await page.getByRole('button', { name: 'Reactivate account' }).click();
    await expect(page.getByText('Player reactivated', { exact: true })).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByRole('button', { name: 'Deactivate account' })).toBeVisible();

    const loginOk = await page.request.post(`${e2eApiBase()}/api/players/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: { email, password },
    });
    expect(loginOk.ok()).toBeTruthy();
  });
});
