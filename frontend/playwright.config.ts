/**
 * Playwright smoke tests over `pnpm dev:mock`: the real UI in Chromium, with
 * Tauri IPC served from the fixture meetings (PARITY_PLAN §3 rule 8).
 *
 *   pnpm test:e2e            run headless; screenshots land in test-results/
 *
 * Benchmarked against Handy's `playwright.config.ts`, which runs the same kind
 * of smoke test against its Vite dev server.
 */
import { defineConfig, devices } from '@playwright/test';

/** `pnpm dev:mock`'s port (`MOCK_PORT` in vite.config.ts; overridable the same way). */
const MOCK_URL = `http://localhost:${process.env.MOCK_PORT ?? '3119'}`;

export default defineConfig({
  testDir: './e2e',
  outputDir: './test-results',
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: MOCK_URL,
    screenshot: 'on',
    trace: 'retain-on-failure',
  },
  // The installed Google Chrome (every Mac and GitHub macOS runner has it), so
  // no Playwright browser download is needed.
  projects: [{ name: 'chrome', use: { ...devices['Desktop Chrome'], channel: 'chrome' } }],
  webServer: {
    command: 'pnpm dev:mock',
    url: MOCK_URL,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
