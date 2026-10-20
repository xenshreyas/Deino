/**
 * TRN-01 in a real browser: the transcript view on a fixture meeting (search,
 * speaker filter, both themes), on a generated two-hour meeting with 5,000
 * lines (the DOM stays small, scrolling and search stay fast, `?t=` lands on
 * its moment), and live on the record screen ("Jump to now").
 */
import { expect, type Page, test } from '@playwright/test';

import {
  LONG_MEETING_ID,
  LONG_MEETING_KEY,
  PHRASE,
  PHRASE_EVERY,
} from '../src/dev/mock-ipc/long-meeting-contract';
import { fixtureUuid } from '../src/dev/mock-ipc/fixture-ids';
import { fixtureMeeting } from './fixtures';

const SEGMENTS = 5_000;
/** A 5,000-line list may keep this many rows in the DOM at most. */
const MAX_DOM_ROWS = 60;

/** Turns on the generated long meeting for this page (before the app loads). */
async function withLongMeeting(page: Page): Promise<void> {
  await page.addInitScript(
    `sessionStorage.setItem(${JSON.stringify(LONG_MEETING_KEY)}, '${String(SEGMENTS)}');`,
  );
}

/**
 * In-page script: milliseconds from typing `phrase` into the search field to
 * the end of the task that handled it (React renders the counter in it).
 * Written as source text, because these specs compile without DOM types.
 */
function searchTiming(phrase: string): string {
  return `(async () => {
    const input = document.querySelector('input[type="search"]');
    const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    const started = performance.now();
    setValue.call(input, ${JSON.stringify(phrase)});
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 0));
    return performance.now() - started;
  })()`;
}

/** In-page condition: the live transcript has room to scroll back. */
const CAN_SCROLL_BACK = `(() => {
  const region = document.querySelector('[role="region"][aria-label="Transcript"]');
  return region !== null && region.scrollHeight - region.clientHeight > 120;
})()`;

const transcript = (page: Page) => page.getByRole('region', { name: 'Transcript' });
const searchBox = (page: Page) => page.getByRole('searchbox', { name: 'Search transcript' });
const counter = (page: Page) => page.getByRole('search').getByRole('status');

test('a meeting transcript reads as speaker turns and searches with speaker filters', async ({
  page,
}) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  // The theme is applied before first paint, so each scheme gets its own load.
  for (const scheme of ['dark', 'light'] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`/meetings/${oneOnOne.id}?tab=transcript`);
    await expect(transcript(page).getByText(oneOnOne.segments[0]?.text ?? '')).toBeVisible();
    await expect(transcript(page).getByText('Maya Chen').first()).toBeVisible();
    await page.screenshot({ path: test.info().outputPath(`transcript-${scheme}.png`) });
  }

  await page.keyboard.press('ControlOrMeta+f');
  await expect(searchBox(page)).toBeFocused();
  await page.keyboard.type('speaker:maya ');
  await expect(page.getByRole('button', { name: 'Remove Maya Chen filter' })).toBeVisible();
  await page.keyboard.type('incident');
  await expect(counter(page)).toHaveText(/^1 of \d+$/);
  await page.keyboard.press('Enter');
  await expect(counter(page)).toHaveText(/^2 of \d+$/);
  await expect(transcript(page).locator('mark[aria-current="true"]')).toBeInViewport();
  await page.screenshot({ path: test.info().outputPath('transcript-search.png') });

  // CAP-04's "Who is …?" opens from a turn's speaker chip, for that line or all of them.
  await searchBox(page).press('Escape');
  await transcript(page).getByRole('button', { name: 'Maya Chen' }).first().click();
  await expect(page.getByRole('heading', { name: 'Who is Maya Chen?' })).toBeVisible();
  // Naming a speaker names all their lines unless the user narrows it.
  await expect(page.getByRole('radio', { name: 'All segments' })).toBeChecked();
  await expect(page.getByText('In this meeting')).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('transcript-rename.png') });
  await page.keyboard.press('Escape');
});

test('a two-hour meeting with 5,000 lines keeps the DOM small and searches fast', async ({
  page,
}) => {
  await withLongMeeting(page);
  await page.goto(`/meetings/${LONG_MEETING_ID}?tab=transcript`);
  const items = transcript(page).getByRole('listitem');
  await expect(items.first()).toBeVisible();
  expect(await items.count()).toBeLessThanOrEqual(MAX_DOM_ROWS);

  // Jump to the end and back: rows render at once and the DOM stays small.
  await items.first().focus();
  await page.keyboard.press('End');
  await expect(page.locator(`[data-bubble="${String(SEGMENTS - 1)}"]`)).toBeFocused();
  expect(await items.count()).toBeLessThanOrEqual(MAX_DOM_ROWS);
  await page.keyboard.press('Home');
  await expect(page.locator('[data-bubble="0"]')).toBeFocused();

  const searchMs: unknown = await page.evaluate(searchTiming(PHRASE));
  await expect(counter(page)).toHaveText(`1 of ${String(SEGMENTS / PHRASE_EVERY)}`);
  expect(searchMs).toBeLessThan(100);

  await searchBox(page).press('Shift+Enter');
  await expect(counter(page)).toHaveText('20 of 20');
  await expect(transcript(page).locator('mark[aria-current="true"]')).toBeInViewport();
  expect(await items.count()).toBeLessThanOrEqual(MAX_DOM_ROWS);
  await page.screenshot({ path: test.info().outputPath('transcript-long.png') });
});

test('a citation link (?segment=) opens the transcript on the cited line', async ({ page }) => {
  const oneOnOne = fixtureMeeting('meeting-fixture-one-on-one');
  const index = 40;
  const cited = oneOnOne.segments[index];
  if (!cited) throw new Error('the one-on-one fixture is too short');
  const segment = fixtureUuid(`segment/${oneOnOne.id}/${String(index)}`);

  await page.goto(`/meetings/${oneOnOne.id}?tab=transcript&segment=${segment}`);

  const marked = transcript(page).locator('[aria-current="location"]');
  await expect(marked).toHaveText(cited.text);
  await expect(marked).toBeInViewport();
  await expect(page).not.toHaveURL(/[?&]segment=/);
});

test('a ?t= link opens the transcript at that moment and marks it', async ({ page }) => {
  await withLongMeeting(page);
  await page.goto(`/meetings/${LONG_MEETING_ID}?t=3600000`);

  const moment = transcript(page).locator('[aria-current="location"]');
  await expect(moment).toBeInViewport();
  await expect(page.getByRole('tab', { name: 'Transcript' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await page.screenshot({ path: test.info().outputPath('transcript-moment.png') });
  // Once shown, the moment leaves the URL, so a reload does not jump again.
  await expect(page).not.toHaveURL(/[?&]t=/);
});

for (const scheme of ['light', 'dark'] as const) {
  test.describe(`live, ${scheme}`, () => {
    // A short window overflows after a few lines (the mock says one every few seconds).
    test.use({ colorScheme: scheme, viewport: { width: 1280, height: 520 } });

    test('a live transcript follows the newest line and offers "Jump to now"', async ({ page }) => {
      test.setTimeout(60_000);
      await page.goto('/');
      await page.getByRole('main').getByRole('button', { name: 'Start recording' }).click();
      const live = transcript(page);
      await expect(live.getByRole('listitem').first()).toBeVisible();
      await page.waitForFunction(CAN_SCROLL_BACK, undefined, { timeout: 40_000 });
      await page.screenshot({ path: test.info().outputPath(`transcript-live-${scheme}.png`) });

      await live.hover();
      await page.mouse.wheel(0, -2_000);
      const jump = page.getByRole('button', { name: 'Jump to now' });
      await expect(jump).toBeVisible();
      await expect(jump).toHaveCSS('opacity', '1');
      await page.screenshot({ path: test.info().outputPath(`transcript-jump-${scheme}.png`) });

      await jump.click();
      await expect(jump).toBeHidden();
      await page.getByRole('main').getByRole('button', { name: 'Stop' }).click();
    });
  });
}
