/**
 * Typed calls to the `retention` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead. The
 * periods themselves are settings (`privacy.retention`).
 */
import { commands } from '@/lib/ipc/generated/retention';
import { unwrap } from '@/lib/ipc/result';

import type { MeetingId, MeetingRetention } from './types';

export const retentionApi = {
  /** When retention deleted the meeting's recording and transcript. */
  status: (meeting: MeetingId): Promise<MeetingRetention> => unwrap(commands.status(meeting)),
};
