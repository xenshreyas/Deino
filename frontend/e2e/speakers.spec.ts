import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('a speaker named from the Participants panel offers to regenerate the notes', async ({
  page,
}) => {
  const standup = fixtureMeeting('meeting-fixture-standup');
  await page.goto(`/meetings/${standup.id}`);
  await expect(page.getByRole('heading', { name: standup.title })).toBeVisible();

  await page.getByRole('button', { name: 'Toggle right panel' }).click();
  await page.getByRole('tab', { name: 'Participants' }).click();
  const speakers = page.getByRole('list', { name: 'Speakers' });
  await speakers.getByRole('button', { name: 'Speaker 5', exact: true }).click();
  await page.getByPlaceholder('Search or type a name').fill('Tom');
  await page.getByRole('option', { name: /Tom Becker/u }).click();

  await expect(speakers.getByRole('button', { name: 'Tom Becker', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Regenerate notes' })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('participants.png') });
});

test('the old Settings → People link opens Advanced → Voices', async ({ page }) => {
  await page.goto('/settings?section=people');

  await expect(page.getByRole('heading', { level: 2, name: 'Advanced' })).toBeVisible();
  await page.getByRole('button', { name: /Maya Chen/u }).click();
  await expect(page.getByRole('button', { name: 'Forget Maya Chen’s voice' })).toBeVisible();
});
