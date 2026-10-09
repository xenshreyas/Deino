import { expect, test } from '@playwright/test';

test('settings show a saved key without revealing it, and switches persist', async ({ page }) => {
  await page.goto('/settings');

  // The mock starts with Claude for notes and its key "in the Keychain".
  await page.getByRole('tab', { name: 'Summary' }).click();
  await expect(
    page.getByText('Saved in the Keychain. Type a new key to replace it.'),
  ).toBeVisible();
  await expect(page.getByPlaceholder('Saved in the Keychain')).toHaveValue('');
  // A saved key is enough to save the model choice without retyping it.
  await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeEnabled();
  await page.screenshot({ path: test.info().outputPath('settings-summary.png') });

  // A switch saves through the settings plugin and survives a tab change.
  await page.getByRole('tab', { name: 'Recordings' }).click();
  const notice = page.getByRole('switch').nth(1);
  await expect(notice).toBeChecked();
  await notice.click();
  await page.getByRole('tab', { name: 'General' }).click();
  await page.getByRole('tab', { name: 'Recordings' }).click();
  await expect(page.getByRole('switch').nth(1)).not.toBeChecked();
});
