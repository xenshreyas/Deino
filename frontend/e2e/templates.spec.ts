import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

/** Screenshots a page in both themes. */
async function shoot(page: Page, name: string) {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    // Let the theme's colour transitions finish before the capture.
    await page.waitForTimeout(400);
    await page.screenshot({
      path: test.info().outputPath(`${name}-${colorScheme}.png`),
      fullPage: true,
    });
  }
  await page.emulateMedia({ colorScheme: 'light' });
}

test('a new template can be created, then picked for a meeting', async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');

  await page.goto('/settings?section=templates');
  await expect(page.getByRole('heading', { name: 'Built-in' })).toBeVisible();
  await shoot(page, 'templates-library');

  await page.getByRole('button', { name: 'New template' }).first().click();
  await page.getByRole('textbox', { name: 'Name' }).fill('Board prep');
  await page.getByRole('textbox', { name: 'Section title' }).fill('Decisions to ask for');
  await page.getByRole('button', { name: 'Add section' }).click();
  await shoot(page, 'templates-editor');
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByText('Saved', { exact: true })).toBeVisible();

  // In-app navigation: a reload would reset the mock, and the new template with it.
  await page.getByRole('link', { name: 'Meetings', exact: true }).click();
  await page.getByText(oneOnOne.title, { exact: true }).click();
  await expect(page).toHaveURL(`/meetings/${oneOnOne.id}`);
  const picker = page.getByRole('button', { name: 'Note template' });
  await expect(picker).toHaveText(/1:1/);
  // The reason is not on the page, only in the menu's header (UX-06).
  await expect(page.getByText('Matched rule: 1:1 in the title')).toHaveCount(0);
  await picker.click();
  await expect(page.getByRole('menu').getByText('Matched rule: 1:1 in the title')).toBeVisible();
  await page.getByRole('menuitem', { name: 'Board prep' }).click();
  await expect(picker).toHaveText(/Board prep/);
  await shoot(page, 'meeting-template-picker');
});

test('Settings → Profile saves and shows what the AI sees', async ({ page }) => {
  await page.goto('/settings?section=profile');
  const role = page.getByRole('textbox', { name: 'Role' });
  await expect(role).toHaveValue('CTO');

  await role.fill('Head of Product');
  await page.getByRole('button', { name: 'Save profile' }).click();

  await expect(page.getByText(/Role: Head of Product at Deino/)).toBeVisible();
  await shoot(page, 'profile');
});
