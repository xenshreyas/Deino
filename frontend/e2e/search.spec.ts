import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

test('⌘K finds a phrase from a transcript and opens the meeting at that moment', async ({
  page,
}) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  // A line seven minutes in, so opening it has to scroll the transcript there.
  const moment = oneOnOne.segments.find((segment) =>
    segment.text.includes('webhook subscriptions for calendar events expire'),
  );
  if (!moment) throw new Error('the one-on-one fixture lost its line about Graph webhooks');
  await page.goto('/');
  await expect(page.getByRole('complementary', { name: 'Sidebar' })).toBeVisible();

  await page.keyboard.press('ControlOrMeta+k');
  const palette = page.getByRole('dialog', { name: 'Command palette' });
  await palette.getByRole('combobox').fill('webhook subscriptions expire');
  const option = palette.getByRole('option', { name: /7:31/ });
  await expect(option).toBeVisible();
  await expect(option.locator('mark').first()).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('search-palette.png') });
  await option.click();

  await expect(page).toHaveURL(
    `/meetings/${oneOnOne.id}?tab=transcript&t=${String(moment.startMs)}`,
  );
  const focused = page.locator('[aria-current="location"]');
  await expect(focused).toBeVisible();
  await expect(focused).toContainText('webhook subscriptions for calendar events expire');
  await page.screenshot({ path: test.info().outputPath('search-moment.png') });
});

test('the results page shows meetings, moments and people with highlighted snippets', async ({
  page,
}) => {
  await page.goto('/search?q=calendar');

  await expect(page.getByRole('tab', { name: /Meetings/ })).toBeVisible();
  await expect(page.locator('mark').first()).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('search-page.png'), fullPage: true });
  await page.getByRole('tab', { name: /Moments/ }).click();
  await expect(page.getByRole('button', { name: /\d+:\d{2}/ }).first()).toBeVisible();
  // Both themes are designed: the highlight token keeps its contrast in the dark.
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.screenshot({ path: test.info().outputPath('search-page-dark.png'), fullPage: true });
  await page.screenshot({
    path: test.info().outputPath('search-page-moments.png'),
    fullPage: true,
  });
});

test('Settings → Advanced → Search downloads the model', async ({ page }) => {
  await page.goto('/settings?section=search');

  await expect(page.getByText('Not downloaded. Search finds exact words only.')).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('search-index-missing.png') });
  await page.getByLabel('Search by meaning').getByRole('button', { name: 'Download' }).click();

  await expect(page.getByText('Downloaded. Search also finds meetings by meaning.')).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('search-index-ready.png') });
});
