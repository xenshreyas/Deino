import { expect, type Page, test } from '@playwright/test';

import { fixtureMeetings } from './fixtures';

/** The fixture events the mock puts under way now and 25 minutes from now. */
function nextTwoTitles(): [string, string] {
  const [first, second] = fixtureMeetings();
  if (!first || !second) throw new Error('the fixtures have fewer than two meetings');
  return [first.calendarEvent.title, second.calendarEvent.title];
}

/** Back to Home inside the app (a reload would reset the mock's state). */
async function goHome(page: Page) {
  await page.getByRole('navigation', { name: 'Main' }).getByRole('link', { name: 'Home' }).click();
}

async function openCalendarSettings(page: Page) {
  await page.goto('/settings');
  await page
    .getByRole('navigation', { name: 'Settings sections' })
    .getByRole('link', { name: 'Calendar' })
    .click();
}

test('Home lists what is coming up from the calendar', async ({ page }) => {
  const [running, soon] = nextTwoTitles();

  await page.goto('/');

  const today = page.getByRole('region', { name: 'Today' });
  await expect(today.getByText(running, { exact: true })).toBeVisible();
  await expect(today.getByText('Now', { exact: true })).toBeVisible();
  await expect(today.getByText(soon, { exact: true })).toBeVisible();
  await expect(today.getByText('in 25m')).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('coming-up.png') });
});

test('Settings → Calendar shows sync, the calendars, and hides one from Coming up', async ({
  page,
}) => {
  const [running] = nextTwoTitles();
  await openCalendarSettings(page);

  await expect(page.getByRole('switch', { name: 'Sync calendars' })).toBeChecked();
  await expect(page.getByText(/^Synced /)).toBeVisible();
  const work = page.getByRole('switch', { name: 'Show Work' });
  await expect(page.getByRole('group', { name: 'Deino fixtures' })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('settings-calendar.png'), fullPage: true });

  await work.click();
  await expect(work).not.toBeChecked();
  await goHome(page);
  await expect(page.getByText('Nothing coming up')).toBeVisible();
  await expect(page.getByText(running, { exact: true })).toHaveCount(0);
});

test('turning sync off replaces Coming up with the prompt to connect', async ({ page }) => {
  await openCalendarSettings(page);
  await page.getByRole('switch', { name: 'Sync calendars' }).click();
  await expect(page.getByRole('switch', { name: 'Sync calendars' })).not.toBeChecked();

  await goHome(page);

  await expect(page.getByText('Connect your calendar')).toBeVisible();
  await page.getByRole('button', { name: 'Set up calendars' }).click();
  await expect(page).toHaveURL('/settings?section=calendar');
});

test('a direct connection connects once its app registration is saved', async ({ page }) => {
  await openCalendarSettings(page);
  const outlook = page.getByRole('group', { name: 'Microsoft Outlook' });
  await expect(outlook.getByText('Add your app registration to connect.')).toBeVisible();
  await expect(outlook.getByRole('button', { name: 'Connect' })).toBeDisabled();

  await outlook.getByLabel('Client ID').fill('00000000-0000-0000-0000-000000000000');
  await outlook.getByLabel('Client ID').blur();
  await outlook.getByRole('button', { name: 'Connect' }).click();

  await expect(outlook.getByText(/^Connected · Synced/)).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath('settings-connectors.png'),
    fullPage: true,
  });
  await outlook.getByRole('button', { name: 'Disconnect' }).click();
  await expect(outlook.getByText('Not connected')).toBeVisible();
});
