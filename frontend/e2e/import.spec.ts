import { expect, type Page, test } from '@playwright/test';

/** Screenshots the page in both themes, then goes back to light. */
async function shoot(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    await page.waitForTimeout(300);
    await page.screenshot({ path: test.info().outputPath(`${name}-${colorScheme}.png`) });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

test('a recording imports from the sidebar and offers its calendar event', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Import recording' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Choose file…' }).click();
  await expect(dialog.getByText('Customer call.mp4')).toBeVisible();
  await dialog.getByLabel('Title').fill('Acme renewal');
  await dialog.getByRole('button', { name: 'Import' }).click();

  await expect(dialog.getByText('Converting the audio…')).toBeVisible();
  await expect(dialog.getByText('Acme renewal')).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Open meeting' })).toBeVisible();
  await shoot(page, 'import-done');
});

test('Settings → Watched folders lists folders and failures', async ({ page }) => {
  await page.goto('/settings?section=watched-folders');
  const folders = page.getByRole('group', { name: 'Folders' });
  await expect(folders.getByText('/Users/me/Movies/Zoom')).toBeVisible();
  const failures = page.getByRole('group', { name: 'Couldn’t import' });
  await expect(failures.getByText('The file has no audio.')).toBeVisible();
  await shoot(page, 'watched-folders');

  await failures.getByRole('button', { name: 'Dismiss' }).click();
  await expect(page.getByText('Nothing has failed.')).toBeVisible();
  await page.getByRole('button', { name: 'Add folder…' }).click();
  await expect(page.getByText('/Users/me/Documents/Voice Memos')).toBeVisible();
});
