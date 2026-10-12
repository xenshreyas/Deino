import { useLinkedEvent } from '@/features/event-links';
import type { MeetingId } from '@/features/meetings';
import { useRecordingState } from '@/features/recording';

/** Whether `meetingId` is the meeting being recorded right now. */
export function useIsLive(meetingId: MeetingId): boolean {
  const { meetingId: recording, phase } = useRecordingState();
  return recording === meetingId && (phase === 'recording' || phase === 'paused');
}

/**
 * Whether the pre-meeting brief still helps (UX-07): while the meeting
 * records, or until its calendar event ends. Afterwards the notes say more.
 */
export function useBriefShown(meetingId: MeetingId): boolean {
  const live = useIsLive(meetingId);
  const event = useLinkedEvent(meetingId).data;
  if (live) return true;
  return event !== undefined && event !== null && Date.parse(event.endAt) > Date.now();
}
