import { expect, test } from '@playwright/test';

test('the Stats page shows the numbers, follows the period and writes the year in review', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('link', { name: 'Stats' }).click();

  await expect(page.getByRole('heading', { name: 'Stats' })).toBeVisible();
  await expect(page.getByRole('group', { name: /last 12 months/ })).toBeVisible();
  await expect(page.getByText('Hours recorded')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'LLM usage' })).toBeVisible();
  await expect(page.getByText(/Costs are estimates/)).toBeVisible();

  const meetings = page.locator('dd').first();
  const year = await meetings.textContent();
  await page.getByRole('radio', { name: '30 days' }).click();
  await expect(meetings).not.toHaveText(year ?? '');

  await page.getByRole('button', { name: 'Write my year in review' }).click();
  await expect(page.getByText(/^Written /)).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('stats.png'), fullPage: true });
});
