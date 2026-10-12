import { expect, type Page, test } from '@playwright/test';

/** Screenshots the page in both themes, then goes back to light. */
async function shootBothThemes(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    // Let the 120 ms colour transitions finish, or buttons keep the old theme.
    await page.waitForTimeout(300);
    await page.screenshot({ path: test.info().outputPath(`${name}-${colorScheme}.png`) });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

/** Opens Settings → Privacy & security. */
async function openPrivacy(page: Page) {
  await page.goto('/settings');
  await page
    .getByRole('navigation', { name: 'Settings sections' })
    .getByRole('link', { name: 'Privacy & security' })
    .click();
}

test('encrypting the library asks first, then asks for a restart', async ({ page }) => {
  await openPrivacy(page);

  const encrypt = page.getByRole('switch', { name: 'Encrypt the library' });
  await expect(encrypt).not.toBeChecked();
  await encrypt.click();
  const dialog = page.getByRole('dialog', { name: 'Encrypt the library?' });
  await expect(dialog).toBeVisible();
  await shootBothThemes(page, 'privacy-encrypt-confirm');
  await dialog.getByRole('button', { name: 'Encrypt at next start' }).click();

  await expect(encrypt).toBeChecked();
  await expect(page.getByText('Restart Deino to encrypt the library.')).toBeVisible();
});

test('the app lock locks the window and a mock Touch ID unlocks it', async ({ page }) => {
  await openPrivacy(page);

  await page.getByRole('switch', { name: 'Lock Deino' }).click();
  await expect(page.getByRole('combobox', { name: 'Lock after' })).toBeVisible();
  await shootBothThemes(page, 'privacy-settings');

  await page.getByRole('button', { name: 'Lock now' }).click();
  await expect(page.getByRole('heading', { name: 'Deino is locked' })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Settings sections' })).toHaveCount(0);
  await shootBothThemes(page, 'lock-screen');

  await page.getByRole('button', { name: 'Unlock with Touch ID' }).click();
  await expect(page.getByRole('link', { name: 'Privacy & security' })).toBeVisible();
});

test('the startup screen shows while the library is converted', async ({ page }) => {
  await page.goto('/?library=encrypting');

  await expect(page.getByRole('heading', { name: 'Encrypting your library…' })).toBeVisible();
  await shootBothThemes(page, 'library-converting');
});
