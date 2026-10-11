/**
 * Mock of `@/lib/ipc/generated/transcript`: each meeting's transcript from
 * the mock tables (the fixtures, the mock recorder's saved lines, and the
 * generated long meeting when asked for), each segment on the speaker the
 * mock speakers say said it, so renames move lines like the Rust app.
 */
import type * as Generated from '@/lib/ipc/generated/transcript';

import { ok } from './command-result';
import { mockTranscripts } from './database';
import { latency } from './latency';
import { mockSpeakers } from './mock-speaker-store';

export const commands = {
  async get(meetingId) {
    await latency();
    return ok(mockTranscripts.get(meetingId, (segment) => mockSpeakers.speakerOf(segment)));
  },
} satisfies typeof Generated.commands;
