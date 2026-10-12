import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

/** Screenshots the page in both themes, then goes back to light. */
async function shoot(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    // Let the theme's colour transition (≤ 200 ms) settle before the shot.
    await page.waitForTimeout(300);
    await page.screenshot({ path: test.info().outputPath(`${name}-${colorScheme}.png`) });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

test('re-linking a meeting to another event updates its attendees and offers to regenerate', async ({
  page,
}) => {
  const board = fixtureMeeting('meeting-fixture-board-update');
  const planning = fixtureMeeting('meeting-fixture-planning');
  await page.goto(`/meetings/${board.id}`);
  await expect(page.getByRole('heading', { name: board.title })).toBeVisible();

  // Details shows the linked event; its picker suggests the day before.
  await page.getByRole('button', { name: 'Toggle right panel' }).click();
  await page.getByRole('tab', { name: 'Details' }).click();
  await page.getByRole('button', { name: `Change calendar event: ${board.title}` }).click();
  const picker = page.getByRole('dialog');
  await expect(picker.getByText('Around this meeting')).toBeVisible();
  await expect(picker.getByRole('option', { name: /Q4 planning/u })).toBeVisible();
  await shoot(page, 'event-picker');

  // Re-linking takes the event's title and invitees, and the notes ask to regenerate.
  await picker.getByRole('option', { name: /Q4 planning/u }).click();
  await expect(page.getByRole('heading', { name: planning.title })).toBeVisible();
  await expect(
    page.getByRole('button', { name: /Change calendar event: Q4 planning/u }),
  ).toBeVisible();
  const banner = page.getByRole('status').filter({ hasText: 'Things changed' });
  await expect(banner).toContainText('the calendar event changed and attendees changed');
  await expect(banner.getByRole('button', { name: 'Regenerate' })).toBeVisible();

  // Same title and people as the planning meeting the day before: "Previous in series".
  const event = page.getByRole('region', { name: 'Calendar event' });
  await expect(event.getByText(planning.attendees[0]?.name ?? '')).toBeVisible();
  await expect(page.getByRole('region', { name: 'Previous in series' })).toContainText(
    planning.title,
  );
  await shoot(page, 'event-relinked');

  // Unlinking leaves the title and offers "Link event" again.
  await page.getByRole('button', { name: /Change calendar event/u }).click();
  await page.getByRole('dialog').getByRole('option', { name: 'Unlink event' }).click();
  await expect(page.getByRole('button', { name: 'Link event' })).toBeAttached();
  await expect(page.getByRole('heading', { name: planning.title })).toBeVisible();
});

test('a recording links the event under way, and its picker works mid-recording', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('main').getByRole('button', { name: 'Start recording' }).click();

  const chip = page.getByRole('button', { name: /Change calendar event/u });
  await expect(chip).toBeVisible();
  await chip.click();
  const picker = page.getByRole('dialog');
  await expect(picker.getByText('Linked', { exact: true })).toBeVisible();
  await expect(picker.getByText('Around this meeting')).toBeVisible();
  await shoot(page, 'event-recording');
});
