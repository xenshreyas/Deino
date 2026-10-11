/** Typed calls to the `transcript` plugin, the feature's only commands. */
import { commands } from '@/lib/ipc/generated/transcript';
import { unwrap } from '@/lib/ipc/result';

import type { MeetingId, Transcript } from './types';

export const transcriptApi = {
  /** The meeting's saved segments in spoken order, with its named speakers. */
  get: (meetingId: MeetingId): Promise<Transcript> => unwrap(commands.get(meetingId)),
};
