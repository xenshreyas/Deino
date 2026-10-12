import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

/** TRN-02: edit in place, delete with Undo, find and replace with counts first. */
const ONE_ON_ONE = fixtureMeeting('meeting-fixture-one-on-one');

/** A line of the transcript by the start of its text. */
function line(page: Page, text: string) {
  return page.getByRole('listitem').filter({ hasText: text }).first();
}

/** The one-on-one's first line, which every test starts from. */
function firstLine(): string {
  const first = ONE_ON_ONE.segments[0];
  if (first === undefined) throw new Error('the one-on-one fixture has no lines');
  return first.text;
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1360, height: 860 });
  await page.goto(`/meetings/${ONE_ON_ONE.id}?tab=transcript`);
  await expect(line(page, firstLine().slice(0, 25))).toBeVisible();
});

test('a double-click edits a line in place and the notes offer to regenerate', async ({ page }) => {
  const bubble = line(page, firstLine().slice(0, 25));
  await bubble.dblclick();
  const field = page.getByRole('textbox', { name: 'Edit' });
  await expect(field).toBeFocused();
  await page.screenshot({ path: test.info().outputPath('edit-inline.png') });
  await field.fill('Morning! Quick one today, I have a hard stop.');
  await field.press('Enter');

  await expect(line(page, 'Morning! Quick one today')).toBeVisible();
  await line(page, 'Morning! Quick one today').hover();
  await expect(page.getByText('· Edited').first()).toBeVisible();
  await expect(page.getByText('Transcript changed:')).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('edit-banner.png') });
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.waitForTimeout(400);
  await page.screenshot({ path: test.info().outputPath('edit-banner-dark.png') });
});

test('Delete removes a line, and Undo brings it back', async ({ page }) => {
  const text = firstLine().slice(0, 25);
  const bubble = line(page, text);
  await bubble.hover();
  await bubble.locator('..').getByRole('button', { name: 'Delete' }).click();

  await expect(line(page, text)).toHaveCount(0);
  const toast = page.getByText('Line deleted');
  await expect(toast).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('delete-undo.png') });
  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(line(page, text)).toBeVisible();
});

test('find and replace counts the matches first, then replaces them', async ({ page }) => {
  await page.getByRole('searchbox', { name: 'Search transcript' }).fill('calendar');
  await page.getByRole('button', { name: 'Find and replace' }).click();
  await page.getByRole('textbox', { name: 'Replace with' }).fill('Calendar™');
  await page.getByRole('radio', { name: 'All meetings' }).click();
  await page.getByRole('button', { name: 'Replace all' }).click();

  const confirm = page.getByRole('button', { name: /^Replace \d+ match/ });
  await expect(confirm).toBeEnabled();
  await expect(page.getByText(/matches? in \d+ lines? across \d+ meetings?/)).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('replace-preview.png') });
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.waitForTimeout(400);
  await page.screenshot({ path: test.info().outputPath('replace-preview-dark.png') });
  await page.emulateMedia({ colorScheme: 'light' });
  await confirm.click();

  await expect(page.getByText(/^Replaced \d+ match/)).toBeVisible();
  await expect(page.getByRole('listitem').filter({ hasText: 'Calendar™' }).first()).toBeVisible();
});

test('the toolbar folds rare actions into ⋯ when the column is narrow', async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 800 });
  const bubble = line(page, firstLine().slice(0, 25));
  await bubble.hover();
  const toolbar = bubble.locator('..').getByRole('toolbar');
  await expect(toolbar.getByRole('button', { name: 'More actions' })).toBeVisible();
  await toolbar.getByRole('button', { name: 'More actions' }).click();
  await expect(page.getByRole('menuitem', { name: /Edit/ })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('toolbar-overflow.png') });
});
