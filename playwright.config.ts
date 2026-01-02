import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for Yew frontend E2E testing
 * @see https://playwright.dev/docs/test-configuration
 */
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
  reporter: [
    ['html', { outputFolder: '_build/playwright-report' }],
    ['junit', { outputFile: '_build/test-results/e2e-results.xml' }],
  ],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: process.env.FRONTEND_URL || 'http://localhost:50023',
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
  webServer: {
    command: './scripts/start-e2e-docker.sh',
    url: 'http://localhost:50023',
    reuseExistingServer: !process.env.CI,
    timeout: 300 * 1000, // 5 minutes for first build
    stdout: 'pipe',
    stderr: 'pipe',
  },
});

