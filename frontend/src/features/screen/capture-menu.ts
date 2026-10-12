/**
 * The record panel's window or screen picker (CAP-09): a native menu, like
 * CAP-10's microphone menu, so the small transparent panel does not clip
 * it and the call keeps focus. It lists the screens and the other apps'
 * windows; picking one captures it for the recording from now on (a
 * manual start included). Needs Tauri; in a plain browser it does nothing.
 */
import { isTauri } from '@tauri-apps/api/core';
import { CheckMenuItem, Menu, MenuItem } from '@tauri-apps/api/menu';

import { screenApi } from './api';
import { screenCopy } from './copy';
import type { CaptureSource, MeetingId, TargetChoice } from './types';

/** Whether two sources are the same window or screen. */
export function sameSource(a: CaptureSource | null, b: CaptureSource): boolean {
  return a !== null && a.kind === b.kind && a.id === b.id;
}

/** The menu's items: one per choice, the current one checked. */
async function items(
  choices: readonly TargetChoice[],
  current: CaptureSource | null,
  onPick: (source: CaptureSource) => void,
) {
  if (choices.length === 0) {
    return [await MenuItem.new({ text: screenCopy.picker.none, enabled: false })];
  }
  return Promise.all(
    choices.map((choice) =>
      CheckMenuItem.new({
        text: choice.label,
        checked: sameSource(current, choice.source),
        action: () => {
          onPick(choice.source);
        },
      }),
    ),
  );
}

/** Pops up the picker for the recording of `meetingId`, `current` checked;
 * a pick starts its capture, then `onPicked` hears of it. */
export async function showCaptureMenu(
  meetingId: MeetingId,
  current: CaptureSource | null,
  onPicked: (source: CaptureSource) => void,
): Promise<void> {
  if (!isTauri()) return;
  const choices = await screenApi.captureChoices().catch((): TargetChoice[] => []);
  const pick = (source: CaptureSource) => {
    screenApi
      .captureSource(meetingId, source)
      .then(() => {
        onPicked(source);
      })
      .catch((error: unknown) => {
        console.error('[screen] Could not capture the picked window', error);
      });
  };
  const menu = await Menu.new({ items: await items(choices, current, pick) });
  await menu.popup();
}
