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

test('Settings → Integrations connects the sandbox and lists the planned providers as WIP', async ({
  page,
}) => {
  await page.goto('/settings?section=integrations');

  const planned = page.getByRole('list', { name: 'On the way' });
  await expect(planned.getByText('Slack')).toBeVisible();
  await expect(planned.getByRole('button', { name: 'Connect Notion' })).toBeDisabled();
  await expect(planned.getByText('WIP').first()).toBeVisible();
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
