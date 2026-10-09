import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('opening a meeting from the list shows its transcript tab', async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  const [first, second] = oneOnOne.segments;
  if (!first || !second) throw new Error('the one-on-one fixture has no transcript');

  await page.goto('/meetings');
  await page.getByText(oneOnOne.title, { exact: true }).click();
  await expect(page).toHaveURL(`/meetings/${oneOnOne.id}`);
  await expect(page.getByRole('heading', { name: oneOnOne.title })).toBeVisible();

  await page.getByRole('tab', { name: 'Transcript' }).click();
  await expect(page).toHaveURL(`/meetings/${oneOnOne.id}?tab=transcript`);
  await expect(page.getByText(first.text)).toBeVisible();
  await expect(page.getByText(second.text)).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('meeting.png') });
});
