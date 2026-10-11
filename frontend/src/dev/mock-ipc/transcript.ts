/**
 * Mock of `@/lib/ipc/generated/transcript`: each meeting's transcript from
 * the mock tables (the fixtures, the mock recorder's saved lines, and the
 * generated long meeting when asked for).
 */
import type * as Generated from '@/lib/ipc/generated/transcript';

import { ok } from './command-result';
import { mockTranscripts } from './database';
import { latency } from './latency';

export const commands = {
  async get(meetingId) {
    await latency();
    return ok(mockTranscripts.get(meetingId));
  },
} satisfies typeof Generated.commands;
