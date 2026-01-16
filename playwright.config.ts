import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for Yew frontend E2E testing
 * @see https://playwright.dev/docs/test-configuration
 */

// Check if we should use production containers (set by run-tests-setup-prod.sh)
const USE_PRODUCTION_CONTAINERS = process.env.USE_PRODUCTION_CONTAINERS === '1';
const PLAYWRIGHT_BASE_URL = process.env.PLAYWRIGHT_BASE_URL;

// Debug logging for configuration
if (USE_PRODUCTION_CONTAINERS) {
  console.log('[Playwright Config] Using production containers - webServer disabled');
  console.log(`[Playwright Config] PLAYWRIGHT_BASE_URL=${PLAYWRIGHT_BASE_URL}`);
} else {
  console.log('[Playwright Config] Using E2E containers via webServer');
}

export default defineConfig({
  testDir: './testing/e2e',
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  /* When using production containers, don't open HTML report (script continues immediately) */
  reporter: USE_PRODUCTION_CONTAINERS ? [
    ['html', { outputFolder: '_build/playwright-report', open: 'never' }],
    ['junit', { outputFile: '_build/test-results/e2e-results.xml' }],
  ] : [
    ['html', { outputFolder: '_build/playwright-report' }],
    ['junit', { outputFile: '_build/test-results/e2e-results.xml' }],
  ],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    /* When USE_PRODUCTION_CONTAINERS=1, use PLAYWRIGHT_BASE_URL (set by run-tests-setup-prod.sh) */
    /* Otherwise, use FRONTEND_URL or default to E2E port 50023 */
    baseURL: process.env.PLAYWRIGHT_BASE_URL || process.env.FRONTEND_URL || 'http://localhost:50023',
    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',
    /* Take screenshot on failure */
    screenshot: 'only-on-failure',
  },

  /* Configure projects for major browsers */
  /* Note: Only Chromium is enabled by default. To test on other browsers, run:
   *   npx playwright install firefox webkit
   * Then uncomment the browser projects below.
   */
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },

    // Uncomment after running: npx playwright install firefox
    // {
    //   name: 'firefox',
    //   use: { ...devices['Desktop Firefox'] },
    // },

    // Uncomment after running: npx playwright install webkit
    // {
    //   name: 'webkit',
    //   use: { ...devices['Desktop Safari'] },
    // },

    /* Test against mobile viewports. */
    // Mobile Chrome uses Chromium engine, so it works without additional install
    {
      name: 'Mobile Chrome',
      use: { ...devices['Pixel 5'] },
    },
    // Uncomment after running: npx playwright install webkit
    // {
    //   name: 'Mobile Safari',
    //   use: { ...devices['iPhone 12'] },
    // },
  ],

  /* Run Docker containers before starting the tests */
  /* NOTE: When running with run-tests-setup-prod.sh, production containers are already running.
   *       Set USE_PRODUCTION_CONTAINERS=1 to skip webServer and use existing containers.
   *       Otherwise, webServer will start E2E containers for standalone Playwright runs.
   */
  webServer: USE_PRODUCTION_CONTAINERS ? undefined : {
    command: './scripts/start-e2e-docker.sh',
    url: 'http://localhost:50023',
    reuseExistingServer: !process.env.CI,
    timeout: 300 * 1000, // 5 minutes for first build
    stdout: 'pipe',
    stderr: 'pipe',
  },
});

