import { expect, type Page, test } from '@playwright/test';

/** Opens a settings page from the settings nav. */
async function openSection(page: Page, name: string) {
  await page
    .getByRole('navigation', { name: 'Settings pages' })
    .getByRole('link', { name })
    .click();
}

test('settings show a saved key without revealing it, and switches persist', async ({ page }) => {
  await page.goto('/settings');

  // The mock starts with Claude for notes and its key "in the Keychain".
  await openSection(page, 'Notes');
  await expect(page.getByText('Key saved')).toBeVisible();
  await page.getByRole('button', { name: 'Other providers' }).click();
  await expect(
    page.getByText('Saved in the Keychain. Type a new key to replace it.'),
  ).toBeVisible();
  await expect(page.getByPlaceholder('Saved in the Keychain')).toHaveValue('');
  // A saved key is enough to save the model choice without retyping it.
  await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeEnabled();
  await page.screenshot({ path: test.info().outputPath('settings-summary.png') });

  // A switch saves through the settings plugin and survives a page change.
  await openSection(page, 'Recording');
  const panel = page.getByRole('switch', { name: 'Show the record panel' });
  await expect(panel).toBeChecked();
  await panel.click();
  await openSection(page, 'General');
  await openSection(page, 'Recording');
  await expect(page.getByRole('switch', { name: 'Show the record panel' })).not.toBeChecked();
});

test('Privacy shows what left the Mac, Advanced the services in use', async ({ page }) => {
  await page.goto('/settings');

  await openSection(page, 'Privacy');
  await expect(page.getByRole('switch', { name: 'Offline mode' })).not.toBeChecked();
  // The log names hosts and sizes, never a URL.
  await expect(page.getByRole('cell', { name: 'cas-bridge.xethub.hf.co' })).toBeVisible();
  await expect(page.getByText('POST · Blocked')).toBeVisible();

  await openSection(page, 'Advanced');
  await expect(page.getByRole('switch', { name: 'Allow Anthropic' })).toBeChecked();
  const slack = page.getByRole('switch', { name: 'Allow Slack' });
  await expect(slack).not.toBeChecked();
  await slack.click();
  await expect(slack).toBeChecked();
  await page.screenshot({ path: test.info().outputPath('settings-network.png'), fullPage: true });
});
