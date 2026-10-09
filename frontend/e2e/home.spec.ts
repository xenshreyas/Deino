import { expect, test } from '@playwright/test';

import { fixtureMeetings } from './fixtures';

test('the Meetings page lists every fixture meeting', async ({ page }) => {
  await page.goto('/');
  await page
    .getByRole('navigation', { name: 'Main' })
    .getByRole('link', { name: 'Meetings' })
    .click();

  await expect(page).toHaveURL('/meetings');
  for (const { title } of fixtureMeetings()) {
    await expect(page.getByText(title, { exact: true })).toBeVisible();
  }
  await page.screenshot({ path: test.info().outputPath('meetings.png') });
});
