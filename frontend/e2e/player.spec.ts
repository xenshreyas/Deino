import { expect, type Page, test } from '@playwright/test';

import { fixtureMeeting } from './fixtures';

/**
 * TRN-03 acceptance: the standup fixture's audio (its first ~53 s) has each
 * line spoken inside its own known slot, so a seek to a line's `startMs`
 * must land the media element there: within ±200 ms.
 */
const SEEK_TOLERANCE_MS = 200;

/** What the tests read of the page's `<audio>` (the e2e project has no DOM types). */
interface MediaProbe {
  currentTime: number;
  readyState: number;
  playbackRate: number;
  addEventListener: (type: 'seeked', listener: () => void, options: { once: boolean }) => void;
}
const STANDUP = fixtureMeeting('meeting-fixture-standup');

/** The hidden media element's position, in milliseconds. */
async function playhead(page: Page): Promise<number> {
  return page.locator('audio').evaluate((audio: MediaProbe) => audio.currentTime * 1000);
}

/** Waits until the media element has its metadata (it can seek). */
async function loaded(page: Page): Promise<void> {
  await expect
    .poll(() => page.locator('audio').evaluate((audio: MediaProbe) => audio.readyState))
    .toBeGreaterThanOrEqual(1);
}

/** The lines inside the recorded clip, at least a second from its end. */
function playableLines() {
  const clipEnd = STANDUP.audio?.clipEndMs ?? 0;
  return STANDUP.segments.filter((segment) => segment.endMs < clipEnd - 1_000);
}

/** Clicks `target`, then measures how far the playhead landed from `expected`. */
async function seekError(page: Page, click: () => Promise<void>, expected: number) {
  await click();
  await expect.poll(() => playhead(page)).toBeGreaterThan(expected - SEEK_TOLERANCE_MS);
  return Math.abs((await playhead(page)) - expected);
}

/**
 * Seeks during playback with "Play from here", reading the position the
 * element reports in its `seeked` event (before playback moves it on).
 */
async function whilePlaying(page: Page): Promise<number[]> {
  await page.getByRole('button', { name: 'Play', exact: true }).click();
  const errors: number[] = [];
  for (const line of playableLines().slice(2, 7)) {
    const landed = page.locator('audio').evaluate(
      (audio: MediaProbe) =>
        new Promise<number>((resolve) => {
          audio.addEventListener('seeked', () => resolve(audio.currentTime * 1000), { once: true });
        }),
    );
    const bubble = page
      .getByRole('listitem')
      .filter({ hasText: line.text.slice(0, 30) })
      .first();
    await bubble.hover();
    // The toolbar sits beside the bubble, in the same wrapper.
    await bubble.locator('..').getByRole('button', { name: 'Play from here' }).click();
    errors.push(Math.abs((await landed) - line.startMs));
  }
  return errors;
}

test('the docked player seeks to lines and words within ±200 ms of their known times', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1360, height: 860 });
  await page.goto(`/meetings/${STANDUP.id}?tab=transcript`);
  await expect(page.getByRole('group', { name: 'Recording' })).toBeVisible();
  await loaded(page);

  const errors: number[] = [];
  for (const line of playableLines().slice(0, 8)) {
    const bubble = page
      .getByRole('listitem')
      .filter({ hasText: line.text.slice(0, 30) })
      .first();
    // The bubble's padding is not a word: the click seeks to the line's start.
    const padding = () => bubble.click({ position: { x: 3, y: 3 } });
    errors.push(await seekError(page, padding, line.startMs));
    const word = line.words[Math.floor(line.words.length / 2)];
    if (word === undefined) continue;
    const span = bubble.locator(`[data-at="${String(word.startMs)}"]`);
    errors.push(await seekError(page, () => span.click(), word.startMs));
  }
  errors.push(...(await whilePlaying(page)));
  const worst = Math.max(...errors);
  test.info().annotations.push({ type: 'seek-error-ms', description: worst.toFixed(1) });
  console.log(`seek error over ${String(errors.length)} seeks: max ${worst.toFixed(1)} ms`);
  expect(worst).toBeLessThanOrEqual(SEEK_TOLERANCE_MS);
  await page.screenshot({ path: test.info().outputPath('player-seek.png') });
});

test('a ?t= link seeks the player, Space plays and lights the line being said', async ({
  page,
}) => {
  const line = playableLines()[4];
  if (line === undefined) throw new Error('the standup fixture has too few recorded lines');
  await page.setViewportSize({ width: 1360, height: 860 });
  await page.goto(`/meetings/${STANDUP.id}?tab=transcript&t=${String(line.startMs)}`);
  await loaded(page);
  await expect.poll(() => playhead(page)).toBeGreaterThan(line.startMs - SEEK_TOLERANCE_MS);
  expect(Math.abs((await playhead(page)) - line.startMs)).toBeLessThanOrEqual(SEEK_TOLERANCE_MS);

  await page.locator('body').click({ position: { x: 5, y: 5 } });
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Pause' })).toBeVisible();
  await expect(page.locator('.bg-highlight').first()).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('player-playing.png') });
  await page.emulateMedia({ colorScheme: 'dark' });
  // Let the theme's colour transitions finish before the picture.
  await page.waitForTimeout(400);
  await page.screenshot({ path: test.info().outputPath('player-playing-dark.png') });
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Play', exact: true })).toBeVisible();
});

test('the track, speed and download menus offer what the meeting has', async ({ page }) => {
  await page.setViewportSize({ width: 1360, height: 860 });
  await page.goto(`/meetings/${STANDUP.id}?tab=transcript`);
  await loaded(page);

  await page.getByRole('button', { name: 'Playback speed' }).click();
  await page.getByRole('menuitem', { name: '1.5×' }).click();
  await expect(page.getByRole('button', { name: 'Playback speed' })).toHaveText('1.5×');
  expect(await page.locator('audio').evaluate((a: MediaProbe) => a.playbackRate)).toBe(1.5);

  await page.getByRole('button', { name: 'Audio track' }).click();
  await expect(page.getByRole('menuitem', { name: 'Them' })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath('player-track-menu.png') });
  await page.getByRole('menuitem', { name: 'Them' }).click();
  await expect(page.getByRole('button', { name: 'Audio track' })).toContainText('Them');
  await loaded(page);
  expect(await page.locator('audio').evaluate((a: MediaProbe) => a.playbackRate)).toBe(1.5);
});

test('a meeting without a recording shows no player', async ({ page }) => {
  await page.goto('/meetings/meeting-fixture-planning?tab=transcript');
  await expect(page.getByRole('listitem').first()).toBeVisible();
  await expect(page.getByRole('group', { name: 'Recording' })).toHaveCount(0);
});
