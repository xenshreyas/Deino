/**
 * Typed calls to the `pipeline` plugin: the only place this feature touches
 * IPC. Each resolves to the data or throws an `IpcError`.
 */
import { commands } from '@/lib/ipc/generated/pipeline';
import { unwrap } from '@/lib/ipc/result';

import type { MeetingId, StageId, StageStatus } from './types';

export const pipelineApi = {
  /** The meeting's stages in run order; empty when the pipeline never ran for it. */
  status: (meetingId: MeetingId): Promise<StageStatus[]> => unwrap(commands.status(meetingId)),
  /** Runs `stage` again, or every stage when `stage` is `null`; resolves to the stages. */
  rerun: (meetingId: MeetingId, stage: StageId | null): Promise<StageStatus[]> =>
    unwrap(commands.rerun(meetingId, stage)),
  /**
   * Transcribes the meeting again with the engine in Settings (echo removed),
   * then runs every later step; resolves to the stages.
   */
  retranscribe: (meetingId: MeetingId): Promise<StageStatus[]> =>
    unwrap(commands.retranscribe(meetingId)),
};
