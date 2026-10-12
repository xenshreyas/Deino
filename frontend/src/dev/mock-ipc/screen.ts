/**
 * Mock of `@/lib/ipc/generated/screen`: Screen Recording is missing until
 * "Open System Settings" is pressed, which grants it, so `pnpm dev:mock`
 * shows both states. `?screen=allowed` starts granted (screenshots). No
 * capture runs and no meeting has snapshots.
 *
 * It imports no feature code: the feature reaches this module through its
 * generated bindings, so importing it back would form a cycle.
 */
import type * as Generated from '@/lib/ipc/generated/screen';

import { ok } from './command-result';
import { latency } from './latency';

let permitted: boolean | null = null;

/** Whether the page asked to start granted (`?screen=allowed`). */
function startsGranted(): boolean {
  return new URLSearchParams(window.location.search).get('screen') === 'allowed';
}

export const commands = {
  async status() {
    await latency();
    permitted ??= startsGranted();
    return ok({
      supported: true,
      permitted,
      capture: { meeting: null, state: 'off' as const, video: false, snapshots: 0 },
    });
  },
  async openScreenRecordingSettings() {
    await latency();
    permitted = true;
    return ok(null);
  },
  async turnOffForMeeting() {
    await latency();
    return ok(false);
  },
  async snapshots() {
    await latency();
    return ok([]);
  },
} satisfies typeof Generated.commands;
