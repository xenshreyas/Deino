/**
 * AI-02-lite and AI-04 (D13): "Ask about this meeting" in the meeting's
 * Chat panel (a cited answer whose chip opens the transcript excerpts, a
 * `/` recipe), and the Recipes page (Built-in and Mine, Remix, Preview).
 * Screenshots in both themes.
 */
import { expect, test } from '@playwright/test';

import { shootBothThemes } from './themes';

const DISCOVERY = '/meetings/meeting-fixture-sales-discovery';

test('a meeting answers questions with citations and runs a / recipe', async ({ page }) => {
  await page.goto(DISCOVERY);
  await page.getByRole('button', { name: 'Toggle right panel' }).click();
  await page.getByRole('tab', { name: 'Chat' }).click();
  const thread = page.getByRole('list', { name: 'Conversation about this meeting' });
  await expect(thread.getByRole('button', { name: 'Source 1' })).toBeVisible();

  await thread.getByRole('button', { name: 'Source 2' }).click();
  await expect(page.getByRole('button', { name: 'Show in transcript' }).first()).toBeVisible();
  await shootBothThemes(page, 'ask-citation');
  await page.keyboard.press('Escape');

  const bar = page.getByRole('textbox', { name: /Ask a question/ });
  await bar.fill('/follow');
  await expect(page.getByRole('listbox', { name: 'Recipes' })).toContainText(
    'Write follow-up email',
  );
  await shootBothThemes(page, 'ask-slash-menu');
  await bar.press('Enter');
  await bar.press('Enter');
  await expect(thread.getByText('/write-follow-up-email')).toBeVisible();
  await expect(thread.getByRole('button', { name: /^Source \d$/ })).toHaveCount(4, {
    timeout: 10_000,
  });
  await shootBothThemes(page, 'ask-thread');
});

test('the Recipes page lists built-ins, remixes one and previews it', async ({ page }) => {
  await page.goto('/recipes');
  await expect(page.getByRole('heading', { name: 'Recipes', level: 1 })).toBeVisible();
  // UX-27: built-ins that need several meetings are hidden until they can run.
  await expect(page.getByRole('button', { name: /Weekly recap/ })).toHaveCount(0);
  await shootBothThemes(page, 'recipes-built-in');

  await page.getByRole('button', { name: 'Remix' }).click();
  await expect(page.getByRole('textbox', { name: 'Name' })).toHaveValue('Coach me (remix)');
  await page.getByRole('button', { name: 'Run' }).click();
  await expect(page.getByRole('button', { name: 'Source 1' })).toBeVisible({ timeout: 10_000 });
  await shootBothThemes(page, 'recipes-mine-preview');
});
