/**
 * Mock of `@/lib/ipc/generated/transcript`: each meeting's transcript from
 * the mock tables (the fixtures, the mock recorder's saved lines, and the
 * generated long meeting when asked for), each segment on the speaker the
 * mock speakers say said it, so renames move lines like the Rust app; and
 * the TRN-02 edits (`mock-transcript-edits.ts`).
 */
import type * as Generated from '@/lib/ipc/generated/transcript';

import { ok } from './command-result';
import { mockTranscripts } from './database';
import { latency } from './latency';
import { mockSpeakers } from './mock-speaker-store';
import { editLine, previewReplace, replaceAll, setDeleted } from './mock-transcript-edits';

export const commands = {
  async get(meetingId) {
    await latency();
    return ok(mockTranscripts.get(meetingId, (segment) => mockSpeakers.speakerOf(segment)));
  },
  async edit(segmentId, text) {
    await latency();
    return editLine(segmentId, text);
  },
  async delete(segmentId) {
    await latency();
    return setDeleted(segmentId, true);
  },
  async restore(segmentId) {
    await latency();
    return setDeleted(segmentId, false);
  },
  async preview(request) {
    await latency();
    return previewReplace(request);
  },
  async replace(request, replacement) {
    await latency();
    return replaceAll(request, replacement);
  },
} satisfies typeof Generated.commands;
