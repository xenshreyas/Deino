/**
 * Mock of `@/lib/ipc/generated/hotkeys`: the three hotkeys in memory, on
 * their defaults. Like Rust, a combination another hotkey has is refused
 * with `conflict`; there is no other app to hold one, and the finer rules
 * (two modifiers, macOS's own shortcuts) are Rust's alone.
 */
import type * as Generated from '@/lib/ipc/generated/hotkeys';

import { failure, ok } from './command-result';
import { latency } from './latency';

const LABELS: Record<Generated.HotkeyAction, string> = {
  record: 'Start or stop recording',
  quick_note: 'Quick note',
  ask: 'Ask',
};

const DEFAULTS: Record<Generated.HotkeyAction, string> = {
  record: 'Ctrl+Alt+Cmd+R',
  quick_note: 'Ctrl+Alt+Cmd+N',
  ask: 'Ctrl+Alt+Cmd+J',
};

let hotkeys: Generated.Hotkey[] = (Object.keys(DEFAULTS) as Generated.HotkeyAction[]).map(
  (action) => ({
    action,
    label: LABELS[action],
    accelerator: DEFAULTS[action],
    defaultAccelerator: DEFAULTS[action],
    state: { kind: 'active' },
  }),
);

/** The hotkey other than `action` that already has `accelerator`. */
function holder(action: Generated.HotkeyAction, accelerator: string): Generated.Hotkey | undefined {
  return hotkeys.find((hotkey) => hotkey.action !== action && hotkey.accelerator === accelerator);
}

export const commands = {
  async list() {
    await latency();
    return ok(hotkeys);
  },
  async set(action: Generated.HotkeyAction, accelerator: string | null) {
    await latency();
    const taken = accelerator === null ? undefined : holder(action, accelerator);
    if (taken !== undefined) {
      return failure<Generated.Hotkey[]>({
        code: 'conflict',
        message: `already used for “${taken.label}”`,
      });
    }
    hotkeys = hotkeys.map((hotkey) =>
      hotkey.action === action
        ? { ...hotkey, accelerator, state: { kind: accelerator === null ? 'off' : 'active' } }
        : hotkey,
    );
    return ok(hotkeys);
  },
} satisfies typeof Generated.commands;
