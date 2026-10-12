import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('the ⌘K palette finds a meeting and opens it', async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  await page.goto('/');
  // The shortcuts bind once the shell has mounted.
  await expect(page.getByRole('complementary', { name: 'Sidebar' })).toBeVisible();

  await page.keyboard.press('ControlOrMeta+k');
  const palette = page.getByRole('dialog', { name: 'Command palette' });
  await expect(palette).toBeVisible();
  await palette.getByRole('combobox').fill(oneOnOne.title);
  await page.screenshot({ path: test.info().outputPath('palette.png') });
  await palette.getByRole('option', { name: oneOnOne.title }).first().click();

  await expect(page).toHaveURL(`/meetings/${oneOnOne.id}`);
});

test('Settings → General → Theme switches the app to the dark theme', async ({ page }) => {
  await page.goto('/settings?section=appearance');

  await page.getByRole('radio', { name: 'Dark' }).click();

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.screenshot({ path: test.info().outputPath('settings-dark.png') });
});

test('`[` collapses and restores the sidebar', async ({ page }) => {
  await page.goto('/meetings');
  const sidebar = page.getByRole('complementary', { name: 'Sidebar' });
  await expect(sidebar.getByRole('link', { name: 'Home' })).toBeVisible();

  await page.keyboard.press('[');
  await expect(sidebar.getByRole('link', { name: 'Home' })).toBeHidden();

  await page.keyboard.press('[');
  await expect(sidebar.getByRole('link', { name: 'Home' })).toBeVisible();
});

test('⌘J says Ask Deino is coming soon instead of opening chat (D13)', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('complementary', { name: 'Sidebar' })).toBeVisible();

  await page.keyboard.press('ControlOrMeta+j');

  await expect(page.getByText('Ask Deino is coming soon.')).toBeVisible();
  await expect(page.getByRole('dialog', { name: 'Ask Deino' })).toHaveCount(0);
});
