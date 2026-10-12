import { expect, test } from '@playwright/test';

test('a recording shows its notes and live transcript, and stop opens the meeting', async ({
  page,
}) => {
  await page.goto('/');
  await expect(
    page.getByRole('heading', { level: 1, name: /^Good (morning|afternoon|evening)$/ }),
  ).toBeVisible();

  await page.getByRole('main').getByRole('button', { name: 'Start recording' }).click();

  // The mock recorder hears a scripted call: Them first, then the user ("You").
  const transcript = page.getByRole('region', { name: 'Transcript' });
  await expect(transcript.getByText('Them').first()).toBeVisible();
  await expect(transcript.getByText('You').first()).toBeVisible({ timeout: 15_000 });
  const notes = page.getByRole('textbox', { name: 'My notes' });
  await notes.click();
  await page.keyboard.type('Beta on Monday');
  await expect(page.getByRole('status').filter({ hasText: 'Saved' })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('recording.png') });

  await page.getByRole('main').getByRole('button', { name: 'Stop' }).click();

  await expect(page).toHaveURL(/\/meetings\/meeting-/);
  await page.getByRole('radio', { name: 'My notes' }).click();
  await expect(page.getByText('Beta on Monday', { exact: true })).toBeVisible();
});

test('Join on Home records the event (ORG-02)', async ({ page }) => {
  await page.goto('/');
  const comingUp = page.getByRole('region', { name: 'Coming up' });
  await comingUp.getByRole('button', { name: 'Join' }).first().click();

  // The live meeting replaces Home as soon as the mock recorder starts.
  await expect(page.getByRole('region', { name: 'Transcript' })).toBeVisible({ timeout: 2_000 });
});
