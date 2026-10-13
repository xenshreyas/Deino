import { expect, test } from '@playwright/test';

import { shootBothThemes } from './themes';

test('the reminder window offers Join meeting and closes once recording starts (ORG-02)', async ({
  page,
}) => {
  await page.setViewportSize({ width: 376, height: 176 });
  await page.goto('/meeting-reminder.html');

  const card = page.getByRole('region', { name: 'Meeting starting soon' });
  await expect(card.getByRole('button', { name: /Join meeting/ })).toBeVisible();
  await shootBothThemes(page, 'reminder');

  // Notes without joining are in the caret's menu; the card slides out, then closes.
  await card.getByRole('button', { name: 'More options' }).click();
  await page.getByRole('menuitem', { name: 'Take notes without joining' }).click();
  await expect(card).toHaveCount(0);
});

test('a right-click on a Coming up row opens its menu with Review last meeting', async ({
  page,
}) => {
  await page.goto('/');
  const comingUp = page.getByRole('region', { name: 'Coming up' });
  await comingUp.getByRole('listitem').first().click({ button: 'right' });

  const menu = page.getByRole('menu');
  await expect(menu.getByRole('menuitem', { name: 'Open in Calendar' })).toBeVisible();
  await shootBothThemes(page, 'coming-up-menu');
  await menu.getByRole('menuitem', { name: 'Review last meeting' }).click();

  await expect(page).toHaveURL(/\/meetings\/meeting-fixture-/);
});

test('Settings → Calendar has the auto-record switches, General the per-calendar reminders', async ({
  page,
}) => {
  await page.goto('/settings');
  const nav = page.getByRole('navigation', { name: 'Settings pages' });
  await nav.getByRole('link', { name: 'Calendar' }).click();
  await page.getByRole('button', { name: 'Choose which meetings' }).click();

  const external = page.getByRole('switch', { name: 'External meetings' });
  await external.scrollIntoViewIfNeeded();
  await external.click();
  await expect(external).toBeChecked();
  await nav.getByRole('link', { name: 'General' }).click();
  await page.getByRole('button', { name: 'Which calendars remind' }).click();
  const remind = page.getByRole('switch', { name: 'Remind for Work' });
  await expect(remind).toBeChecked();
  await remind.click();
  await expect(remind).not.toBeChecked();
  await shootBothThemes(page, 'settings-auto-record');
});
