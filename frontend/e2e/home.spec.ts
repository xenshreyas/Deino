import { expect, test } from '@playwright/test';

import { fixtureMeetings } from './fixtures';

test('home lists every fixture meeting in the sidebar', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Expand sidebar' }).click();

  for (const { title } of fixtureMeetings()) {
    await expect(page.getByText(title, { exact: true })).toBeAttached();
  }
  // The newest meeting heads the list.
  await expect(page.getByText('Eng standup', { exact: true })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('home.png') });
});
