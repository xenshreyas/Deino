/**
 * What the record panel asks of its own window (CAP-10): the native
 * microphone menu, and dragging. Both need Tauri; in a plain browser
 * (`pnpm dev:mock`) they do nothing.
 *
 * The microphone picker is a native menu rather than a popover: the panel
 * is a small transparent window, and a menu is not clipped by it, reads as
 * part of macOS and works without the window becoming key (so the call
 * keeps focus).
 */
import { isTauri } from '@tauri-apps/api/core';
import { CheckMenuItem, Menu, PredefinedMenuItem } from '@tauri-apps/api/menu';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { panelCopy } from './copy';
import type { Microphone } from './types';

/** Pops up the microphone menu; `onPick` gets the chosen name (`null`: the default). */
export async function showMicrophoneMenu(
  microphones: readonly Microphone[],
  current: string | null,
  onPick: (name: string | null) => void,
): Promise<void> {
  if (!isTauri()) return;
  const choices = [
    await CheckMenuItem.new({
      text: panelCopy.systemDefault,
      checked: current === null,
      action: () => {
        onPick(null);
      },
    }),
    await PredefinedMenuItem.new({ item: 'Separator' }),
    ...(await Promise.all(
      microphones.map((microphone) =>
        CheckMenuItem.new({
          text: microphone.name,
          checked: current === microphone.name,
          action: () => {
            onPick(microphone.name);
          },
        }),
      ),
    )),
  ];
  const menu = await Menu.new({ items: choices });
  await menu.popup();
}

/** Starts dragging the panel window (the OS moves it until the mouse is up). */
export function dragPanel(): void {
  if (!isTauri()) return;
  getCurrentWindow()
    .startDragging()
    .catch((error: unknown) => {
      console.error('[recording] Could not drag the record panel', error);
    });
}
