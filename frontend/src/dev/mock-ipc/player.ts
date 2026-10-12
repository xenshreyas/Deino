/**
 * Mock of `@/lib/ipc/generated/player`: the fixtures' audio as each
 * meeting's Me and Them tracks (`mock-media.ts`). Download has no save
 * dialog in a browser, so it answers "cancelled".
 */
import type * as Generated from '@/lib/ipc/generated/player';

import { ok } from './command-result';
import { mockMedia } from './database';
import { latency } from './latency';

export const commands = {
  async media(meetingId) {
    await latency();
    return ok(mockMedia.media(meetingId));
  },
  async download() {
    await latency();
    return ok<Generated.DownloadOutcome>('cancelled');
  },
} satisfies typeof Generated.commands;
