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
  // At 1280 px the meeting page has no context rail: the draft waits in Details.
  await expect(page.getByRole('heading', { name: SALES.title })).toBeVisible();
  await expect(page.getByRole('complementary', { name: 'Meeting context' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Toggle right panel' }).click();
  await page.getByRole('tab', { name: 'Details' }).click();
  const block = page.getByRole('region', { name: 'Follow-up email' });
  await expect(block.getByText(/^Draft ready · to \d+ people$/)).toBeVisible();
  await shootBothThemes(page, 'followups-details');

  await block.getByRole('button', { name: 'Open draft' }).click();
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

  await block.getByRole('button', { name: 'Dismiss for good' }).click();
  await expect(block).toBeHidden();
});

test('a wide window shows the context rail: talk time, action items and the draft', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1512, height: 945 });
  await page.goto(`/meetings/${SALES.id}`);
  const rail = page.getByRole('complementary', { name: 'Meeting context' });
  const people = rail.getByRole('region', { name: 'People' });
  await expect(people.getByText(/^\d+%$/).first()).toBeVisible();
  await expect(rail.getByRole('region', { name: 'Follow-up email' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Follow-up email ready' })).toHaveCount(0);
  await shootBothThemes(page, 'context-rail');

  // The right panel's Details shows the same people and draft, so the rail gives way.
  await page.getByRole('button', { name: 'Toggle right panel' }).click();
  await expect(rail).toHaveCount(0);
});
