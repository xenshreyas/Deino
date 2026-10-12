import { expect, type Page, test } from '@playwright/test';

/**
 * TRN-04 and TRN-06 smoke: a selected transcript line gets a menu with
 * Comment, Clip and Copy link; a comment lands in Details' Comments; a
 * clip is named in a dialog and listed in Details' Clips.
 */
const STANDUP = '/meetings/meeting-fixture-standup';

/** Selects the whole transcript line holding `words`. */
async function selectLine(page: Page, words: string): Promise<void> {
  await page.locator('[data-segment-id]', { hasText: words }).first().selectText();
}

test('a comment on selected transcript words shows in Details', async ({ page }) => {
  await page.goto(`${STANDUP}?tab=transcript`);
  await expect(page.locator('[data-segment-id]').first()).toBeVisible();

  await selectLine(page, 'Fernhollow dashboards');
  const menu = page.getByRole('toolbar', { name: 'Selection actions' });
  await expect(menu.getByRole('button', { name: 'Copy link' })).toBeVisible();
  await menu.getByRole('button', { name: 'Comment' }).click();
  await page.getByRole('textbox', { name: 'Add a comment' }).fill('Which dashboards?');
  await page.getByRole('button', { name: 'Comment', exact: true }).last().click();

  await page.getByRole('button', { name: 'Toggle right panel' }).click();
  await page.getByRole('tab', { name: 'Details' }).click();
  const comments = page.getByRole('region', { name: 'Comments' });
  await expect(comments.getByText('Which dashboards?')).toBeVisible();
  await expect(page.getByRole('blockquote').filter({ hasText: 'Fernhollow' })).toBeVisible();
});

test('a clip of selected words is named and listed in Details', async ({ page }) => {
  await page.goto(`${STANDUP}?tab=transcript`);
  await expect(page.locator('[data-segment-id]').first()).toBeVisible();

  await selectLine(page, 'writing the subscription renewal');
  await page
    .getByRole('toolbar', { name: 'Selection actions' })
    .getByRole('button', { name: 'Clip' })
    .click();
  const dialog = page.getByRole('dialog', { name: 'New clip' });
  await dialog.getByRole('textbox').fill('Renewal design');
  await dialog.getByRole('button', { name: 'Create clip' }).click();
  await expect(dialog).toBeHidden();

  await page.getByRole('button', { name: 'Toggle right panel' }).click();
  await page.getByRole('tab', { name: 'Details' }).click();
  const clips = page.getByRole('region', { name: 'Clips' });
  await expect(clips.getByText('Renewal design')).toBeVisible();
  await expect(clips.getByText('Release cut plan')).toBeVisible();
});
