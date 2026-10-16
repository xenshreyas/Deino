import { useQueryClient } from '@tanstack/react-query';

import { events } from '@/lib/ipc/generated/pipeline';
import { useIpcEvent } from '@/lib/ipc/events';

import { pipelineKeys } from '../queries';
import type { MeetingId } from '../types';

/**
 * Follows Rust's typed `PipelineProgress` event for the meeting. A running
 * stage's percent only goes into the cache (`useStagePercent`), with no
 * refetch; a status change clears that stage's percent and refetches the
 * status, so the strip moves from pending to running to done without
 * waiting for anything else.
 */
export function usePipelineProgress(meetingId: MeetingId): void {
  const client = useQueryClient();
  useIpcEvent(events.pipelineProgress, (progress) => {
    if (progress.meetingId !== meetingId) return;
    client.setQueryData(pipelineKeys.percent(meetingId, progress.stage), progress.percent);
    if (progress.percent !== null) return;
    void client.invalidateQueries({ queryKey: pipelineKeys.status(meetingId) });
  });
}
