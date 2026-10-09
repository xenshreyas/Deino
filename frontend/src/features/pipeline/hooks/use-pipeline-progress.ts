import { useQueryClient } from '@tanstack/react-query';

import { events } from '@/lib/ipc/generated/pipeline';
import { useIpcEvent } from '@/lib/ipc/events';

import { pipelineKeys } from '../queries';
import type { MeetingId } from '../types';

/**
 * Refetches the meeting's pipeline status whenever Rust's typed
 * `PipelineProgress` event reports one of its stages changing, so the strip
 * moves from pending to running to done without waiting for anything else.
 */
export function usePipelineProgress(meetingId: MeetingId): void {
  const client = useQueryClient();
  useIpcEvent(events.pipelineProgress, (progress) => {
    if (progress.meetingId !== meetingId) return;
    void client.invalidateQueries({ queryKey: pipelineKeys.status(meetingId) });
  });
}
