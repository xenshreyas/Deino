import { expect, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';
import { shootBothThemes } from './themes';

const SALES = fixtureMeeting('meeting-fixture-sales-discovery');

test('a suggested follow-up goes from Home to the editor, then is dismissed for good', async ({
  page,
}) => {
  await page.goto('/');
  const section = page.getByRole('region', { name: 'Suggested follow-ups' });
  await expect(section.getByRole('link', { name: SALES.title })).toBeVisible();
  await section.scrollIntoViewIfNeeded();
  await shootBothThemes(page, 'followups-home');

  await section.getByRole('link', { name: SALES.title }).click();
  const chip = page.getByRole('button', { name: 'Follow-up email ready' });
  await expect(chip).toBeVisible();
  await shootBothThemes(page, 'followups-chip');

  await chip.click();
  await page.getByRole('menuitem', { name: 'View draft' }).click();
  const editor = page.getByRole('dialog', { name: 'Follow-up email' });
  await expect(editor.getByLabel('To')).toHaveValue(
    'marcus.webb@brightkite-logistics.com, hannah.pike@brightkite-logistics.com',
  );
  await expect(editor.getByText('3 gaps to fill')).toBeVisible();
  // UX-27: sending from Deino is hidden until the mail integrations ship.
  await expect(editor.getByRole('button', { name: /Send via Gmail or Outlook/ })).toHaveCount(0);
  await shootBothThemes(page, 'followups-editor');

  await editor.getByRole('button', { name: 'Cc', exact: true }).click();
  await editor.getByLabel('Cc', { exact: true }).fill('lena@deino.dev');
  await editor.getByRole('button', { name: 'Open in Mail' }).click();
  await page.keyboard.press('Escape');
  await expect(editor).toBeHidden();

  await chip.click();
  await page.getByRole('menuitem', { name: 'Dismiss for good' }).click();
  await expect(chip).toBeHidden();
});
