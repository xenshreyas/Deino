import { expect, test } from '@playwright/test';

import { shootBothThemes } from './themes';

test('the reminder window offers Join & record and closes once recording starts (ORG-02)', async ({
  page,
}) => {
  await page.setViewportSize({ width: 376, height: 176 });
  await page.goto('/meeting-reminder.html');

  const card = page.getByRole('region', { name: 'Meeting starting soon' });
  await expect(card.getByRole('button', { name: 'Join & record' })).toBeVisible();
  await expect(card.getByRole('button', { name: 'Record only' })).toBeVisible();
  await shootBothThemes(page, 'reminder');

  await card.getByRole('button', { name: 'Record only' }).click();
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

test('Settings → Calendar has the auto-record switches and per-calendar reminders', async ({
  page,
}) => {
  await page.goto('/settings');
  await page
    .getByRole('navigation', { name: 'Settings sections' })
    .getByRole('link', { name: 'Calendar' })
    .click();

  const external = page.getByRole('switch', { name: 'External meetings' });
  await external.scrollIntoViewIfNeeded();
  await external.click();
  await expect(external).toBeChecked();
  const remind = page.getByRole('switch', { name: 'Remind for Work' });
  await expect(remind).toBeChecked();
  await remind.click();
  await expect(remind).not.toBeChecked();
  await shootBothThemes(page, 'settings-auto-record');
});
