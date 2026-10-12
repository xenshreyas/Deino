/**
 * What the record panel asks of its own window (CAP-10): the pill's native
 * ⋯ menu, and dragging. Both need Tauri; in a plain browser (`pnpm
 * dev:mock`) they do nothing.
 *
 * The ⋯ menu (UX-29: the rare device switches, out of the pill) is a native
 * menu rather than a popover: the panel is a small transparent window, and
 * a menu is not clipped by it, reads as part of macOS and works without
 * the window becoming key (so the call keeps focus).
 */
import { isTauri } from '@tauri-apps/api/core';
import {
  CheckMenuItem,
  Menu,
  PredefinedMenuItem,
  Submenu,
  type SubmenuOptions,
} from '@tauri-apps/api/menu';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { panelCopy } from './copy';
import type { Microphone } from './types';

/** The microphone submenu's items: the system default, then each microphone. */
async function microphoneItems(
  microphones: readonly Microphone[],
  current: string | null,
  onPick: (name: string | null) => void,
) {
  return [
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
}

/** What the pill's ⋯ menu offers and reports. */
export interface PillMenu {
  microphones: readonly Microphone[];
  /** The microphone in use; `null`: the system default. */
  microphone: string | null;
  onMicrophone: (name: string | null) => void;
  /** The screen picker's items (`captureMenuItems` from the screen feature). */
  screens: NonNullable<SubmenuOptions['items']>;
  /** Whether the call's audio is recorded. */
  systemAudio: boolean;
  onSystemAudio: () => void;
}

/** Pops up the ⋯ menu: Microphone ›, Screen to capture ›, Record the call's audio. */
export async function showPillMenu(menu: PillMenu): Promise<void> {
  if (!isTauri()) return;
  const items = [
    await Submenu.new({
      text: panelCopy.microphone,
      items: await microphoneItems(menu.microphones, menu.microphone, menu.onMicrophone),
    }),
    await Submenu.new({ text: panelCopy.screen, items: menu.screens }),
    await PredefinedMenuItem.new({ item: 'Separator' }),
    await CheckMenuItem.new({
      text: panelCopy.recordCallAudio,
      checked: menu.systemAudio,
      action: menu.onSystemAudio,
    }),
  ];
  await (await Menu.new({ items })).popup();
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
