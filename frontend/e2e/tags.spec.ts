import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('a new tag created from the meeting header lands on the meeting', async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  await page.goto(`/meetings/${oneOnOne.id}`);
  await expect(page.getByRole('heading', { name: oneOnOne.title })).toBeVisible();

  await page.getByRole('button', { name: 'Add tag' }).click();
  await page.getByPlaceholder('Find or create a tag').fill('Q4 roadmap');
  await page.getByRole('option', { name: 'Create “Q4 roadmap”' }).click();
  await page.keyboard.press('Escape');

  const header = page.locator('header').filter({ hasText: oneOnOne.title });
  await expect(header.getByText('Q4 roadmap')).toBeVisible();
  await expect(header.getByRole('button', { name: 'Remove Q4 roadmap' })).toBeAttached();
  await page.screenshot({ path: test.info().outputPath('meeting-tags.png') });
});

test('the tag filter narrows the Meetings page to tagged meetings', async ({ page }) => {
  const interview = fixtureMeeting('meeting-fixture-interview');
  const standup = fixtureMeeting('meeting-fixture-standup');
  await page.goto('/meetings');

  await page.getByRole('button', { name: /^Filter/ }).click();
  await page.getByRole('option', { name: 'Hiring', exact: true }).click();

  await expect(page.getByRole('link', { name: interview.title })).toBeVisible();
  await expect(page.getByRole('link', { name: standup.title })).toBeHidden();
});
