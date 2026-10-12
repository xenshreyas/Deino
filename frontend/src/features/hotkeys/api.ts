/**
 * Typed calls to the `hotkeys` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/hotkeys';
import { unwrap } from '@/lib/ipc/result';

import type { Hotkey, HotkeyAction } from './types';

export const hotkeysApi = {
  /** Every hotkey: its combination, default and whether it works. */
  list: (): Promise<Hotkey[]> => unwrap(commands.list()),
  /**
   * Gives `action` the combination `accelerator`, or turns it off with
   * `null`; resolves to every hotkey. Rejects with `invalid` or `conflict`.
   */
  set: (action: HotkeyAction, accelerator: string | null): Promise<Hotkey[]> =>
    unwrap(commands.set(action, accelerator)),
};
