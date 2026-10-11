import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('enhanced notes cite the transcript, keep edits and ask before replacing them', async ({
  page,
}) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  await page.goto(`/meetings/${oneOnOne.id}`);

  // A meeting with notes opens on "Enhanced": the user's lines in ink, the AI's in grey.
  await expect(page.getByRole('radio', { name: 'Enhanced' })).toHaveAttribute('data-state', 'on');
  const notes = page.getByRole('textbox', { name: 'Enhanced notes' });
  await expect(notes.getByRole('heading', { name: 'Overview' })).toBeVisible();
  await expect(
    notes.locator('[data-origin="user"]', { hasText: 'send her PM draft thurs' }),
  ).toBeVisible();

  // Hovering an AI line shows the magnifier; it opens the cited excerpts.
  const aiLine = notes.locator('li', { has: page.locator('[data-origin="ai"]') }).first();
  await aiLine.hover();
  await page.getByRole('button', { name: 'Show sources' }).click();
  const sources = page.getByRole('dialog');
  await expect(sources.getByText('From the transcript')).toBeVisible();
  await expect(sources.getByRole('button', { name: 'Show in transcript' }).first()).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('enhanced-notes-citation.png') });
  await page.keyboard.press('Escape');
  await expect(sources).toBeHidden();

  // Editing an AI line keeps it AI text, marked edited, and saves.
  await aiLine.locator('[data-origin="ai"]').first().click();
  await page.keyboard.press('End');
  await page.keyboard.type(' Back Monday.');
  await expect(notes.locator('[data-origin="ai"][data-edited="true"]').first()).toBeVisible();
  await expect(page.getByRole('status').filter({ hasText: 'Saved' })).toBeVisible();

  // Regenerating over the edit asks first; agreeing writes version 2.
  await page.getByRole('button', { name: 'Regenerate', exact: true }).first().click();
  const confirm = page.getByRole('dialog', { name: 'Replace your edits?' });
  await expect(confirm).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('enhanced-notes-confirm.png') });
  await confirm.getByRole('button', { name: 'Regenerate' }).click();
  await expect(page.getByRole('button', { name: 'Version history' })).toContainText('Version 2', {
    timeout: 10_000,
  });
  await expect(notes.getByText('Back Monday.')).toHaveCount(0);
});

test('a meeting without enhanced notes offers to write them', async ({ page }) => {
  const standup = fixtureMeeting('meeting-fixture-standup');
  await page.goto(`/meetings/${standup.id}`);

  await page.getByRole('radio', { name: 'Enhanced' }).click();

  await expect(page.getByText('No enhanced notes yet')).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('enhanced-notes-empty.png') });
});
