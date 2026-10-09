import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('notes typed in a meeting are saved and come back after a reload', async ({ page }) => {
  const standup = fixtureMeeting('meeting-fixture-standup');

  await page.goto(`/meetings/${standup.id}`);
  // The standup has no summary, so the Notes tab opens on "My notes".
  await expect(page.getByRole('radio', { name: 'My notes' })).toHaveAttribute('data-state', 'on');
  const editor = page.getByRole('textbox', { name: 'My notes' });
  await expect(editor).toBeVisible();

  await editor.click();
  await page.keyboard.type('# Release cut\n');
  await page.keyboard.type('[] merge the offline list by 3:30');
  await expect(page.getByRole('status').filter({ hasText: 'Saved' })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('notepad.png') });

  await page.reload();

  await expect(page.getByRole('heading', { name: 'Release cut' })).toBeVisible();
  await expect(page.getByText('merge the offline list by 3:30', { exact: true })).toBeVisible();
  await expect(page.getByRole('checkbox')).not.toBeChecked();
});

test('a fixture meeting opens with the notes its user typed', async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');

  await page.goto(`/meetings/${oneOnOne.id}`);
  await page.getByRole('radio', { name: 'My notes' }).click();

  await expect(page.getByRole('heading', { name: 'maya 1:1 10/7' })).toBeVisible();
});
