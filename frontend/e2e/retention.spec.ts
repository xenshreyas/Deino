import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

/** Screenshots the page in both themes, then goes back to light. */
async function shootBothThemes(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    // Let the 120 ms colour transitions finish, or buttons keep the old theme.
    await page.waitForTimeout(300);
    await page.screenshot({ path: test.info().outputPath(`${name}-${colorScheme}.png`) });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

test('retention periods: a longer recording period raises the transcript', async ({ page }) => {
  await page.goto('/settings');
  await page
    .getByRole('navigation', { name: 'Settings sections' })
    .getByRole('link', { name: 'Privacy & security' })
    .click();

  const audio = page.getByRole('combobox', { name: 'Delete recordings after' });
  await audio.click();
  await page.getByRole('option', { name: '7 days' }).click();
  // Recordings for 7 days left transcripts at "Never", which is longer.
  await page.getByRole('combobox', { name: 'Delete transcripts after' }).click();
  await page.getByRole('option', { name: '30 days' }).click();
  await audio.click();
  await page.getByRole('option', { name: '90 days' }).click();

  const transcript = page.getByRole('combobox', { name: 'Delete transcripts after' });
  await expect(transcript).toHaveText('90 days');
  await transcript.click();
  await expect(page.getByRole('option', { name: '30 days' })).toHaveAttribute(
    'aria-disabled',
    'true',
  );
  await page.keyboard.press('Escape');
  await page.getByRole('heading', { name: 'Retention' }).scrollIntoViewIfNeeded();
  await shootBothThemes(page, 'retention-settings');
});

test('a deleted transcript says so, and Regenerate is disabled', async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');

  await page.goto(`/meetings/${oneOnOne.id}?tab=transcript&retention=transcript`);

  await expect(page.getByText('Transcript deleted')).toBeVisible();
  await expect(page.getByRole('button', { name: /Regenerate|Enhance/ })).toBeDisabled();
  await shootBothThemes(page, 'retention-transcript-deleted');
});

test('the record panel shows the pause tip once', async ({ page }) => {
  await page.goto('/record-panel.html?demo=collapsed');

  const tip = page.getByRole('region', { name: 'Pause for sensitive discussions' });
  await expect(tip).toBeVisible();
  await shootBothThemes(page, 'retention-pause-tip');
  await tip.getByRole('button', { name: 'Got it' }).click();
  await expect(tip).toHaveCount(0);
});
