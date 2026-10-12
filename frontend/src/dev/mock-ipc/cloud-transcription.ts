/**
 * Mock of `@/lib/ipc/generated/cloud-transcription` (CAP-14): no ElevenLabs
 * key until one is saved; "Check" accepts any key but `bad`; re-transcribing
 * re-runs the meeting's steps through the pipeline mock and keeps the
 * original, which "Restore" gives back.
 */
import type * as Generated from '@/lib/ipc/generated/cloud-transcription';

import { failure, ok } from './command-result';
import { latency } from './latency';
import { commands as pipeline } from './pipeline';

type MeetingId = Generated.MeetingId;

let key: string | null = null;
/** Meetings whose local transcript is "kept". */
const originals = new Set<MeetingId>();

const missingKey = () =>
  failure<never>({
    code: 'unavailable',
    message: 'Save your ElevenLabs API key in Settings → Transcription first.',
  });

export const commands = {
  async keyStatus() {
    await latency();
    return ok(key === null ? 'missing' : 'stored');
  },
  async saveKey(value: string) {
    await latency();
    if (value.trim() === '')
      return failure<null>({ code: 'invalid', message: 'The key is empty.' });
    key = value.trim();
    return ok(null);
  },
  async checkKey() {
    await latency();
    if (key === null) return missingKey();
    if (key === 'bad') {
      return failure<null>({
        code: 'permission_denied',
        message: 'ElevenLabs refused the API key. Check it and save it again.',
      });
    }
    return ok(null);
  },
  async forgetKey() {
    await latency();
    key = null;
    return ok(null);
  },
  async retranscribe(meetingId: MeetingId) {
    await latency();
    if (key === null) return missingKey();
    originals.add(meetingId);
    return pipeline.rerun(meetingId, null);
  },
  async hasOriginal(meetingId: MeetingId) {
    await latency();
    return ok(originals.has(meetingId));
  },
  async restoreOriginal(meetingId: MeetingId) {
    await latency();
    return ok(originals.delete(meetingId));
  },
} satisfies typeof Generated.commands;
