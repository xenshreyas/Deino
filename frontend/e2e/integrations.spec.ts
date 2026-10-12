import { expect, type Page, test } from '@playwright/test';

/** Saves the page in both themes, then returns to light. */
async function screenshots(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    // Let the theme's colour transitions finish before the capture.
    await page.waitForTimeout(400);
    await page.screenshot({
      path: test.info().outputPath(`${name}-${colorScheme}.png`),
      fullPage: true,
    });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

test('Settings → Integrations connects the sandbox; planned providers stay hidden (UX-27)', async ({
  page,
}) => {
  await page.goto('/settings?section=integrations');

  await expect(page.getByRole('list', { name: 'On the way' })).toHaveCount(0);
  await expect(page.getByText('WIP')).toHaveCount(0);
  await expect(page.getByText('Recently sent')).toBeVisible();
  await screenshots(page, 'integrations');

  const sandbox = page.getByRole('group', { name: 'Sandbox' });
  await sandbox.getByRole('button', { name: 'Connect' }).click();
  await sandbox.getByLabel('Sandbox token').fill('sandbox-token');
  await sandbox.getByRole('button', { name: 'Connect' }).click();

  await expect(sandbox.getByText('Sandbox workspace')).toBeVisible();
  await expect(sandbox.getByText('Connected', { exact: true })).toBeVisible();
  await screenshots(page, 'integrations-connected');

  await sandbox.getByRole('button', { name: 'Disconnect' }).click();
  await expect(sandbox.getByText('Sandbox workspace')).toBeHidden();
  await expect(sandbox.getByRole('button', { name: 'Connect' })).toBeVisible();
});
