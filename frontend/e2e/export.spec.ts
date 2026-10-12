import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('a meeting exports to Markdown from the Share menu', async ({ page }) => {
  const standup = fixtureMeeting('meeting-fixture-standup');
  await page.goto(`/meetings/${standup.id}`);
  await expect(page.getByRole('heading', { name: standup.title })).toBeVisible();

  await page.getByRole('button', { name: 'Share and export' }).click();
  await page.getByRole('menuitem', { name: 'Export…' }).click();

  const dialog = page.getByRole('dialog', { name: 'Export meeting' });
  await expect(dialog.getByRole('radio', { name: /Markdown/ })).toBeChecked();
  await expect(dialog.getByText(`title: "${standup.title}"`)).toBeVisible();
  await expect(dialog.getByText('2026-10-08 Eng standup.md')).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('export-dialog.png') });

  await dialog.getByRole('button', { name: 'Export…' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByText('Saved 2026-10-08 Eng standup.md')).toBeVisible();
});

test('the PDF preview renders the document', async ({ page }) => {
  const standup = fixtureMeeting('meeting-fixture-standup');
  await page.goto(`/meetings/${standup.id}`);
  await page.getByRole('button', { name: 'Share and export' }).click();
  await page.getByRole('menuitem', { name: 'Export…' }).click();

  const dialog = page.getByRole('dialog', { name: 'Export meeting' });
  await dialog.getByRole('radio', { name: /PDF/ }).click();
  const frame = page.frameLocator('iframe[title="Document preview"]');
  await expect(frame.getByRole('heading', { name: standup.title })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('export-pdf-preview.png') });
});

test('Settings → Connections → Export chooses a folder and runs the account export', async ({
  page,
}) => {
  await page.goto('/settings?section=export');
  const autoExport = page.getByRole('group', { name: 'Auto-export' });
  await expect(autoExport.getByText('No folder chosen')).toBeVisible();

  await autoExport.getByRole('button', { name: 'Choose…' }).click();
  await expect(autoExport.getByText('/Users/shreyas/Obsidian/Deino')).toBeVisible();

  await page.getByRole('button', { name: 'Export all meetings…' }).click();
  await expect(page.getByText('Saved Deino export.zip.')).toBeVisible({ timeout: 10_000 });
  await page.screenshot({ path: test.info().outputPath('export-settings.png'), fullPage: true });
});
