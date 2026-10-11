/**
 * The Transcript tab (TRN-01): the meeting's transcript view, live while
 * this meeting is being recorded, opened at a linked moment (`?t=`). Its
 * tools are "Copy transcript" and, while the inherited session still owns
 * them, the recording folder and re-transcription buttons.
 */
import type { MeetingId } from '@/features/meetings';
import { useShareActions } from '@/features/export';
import { useRecordingState } from '@/features/recording';
import { TranscriptPanel, transcriptCopy } from '@/features/transcript';
import { Copy } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { useLegacyMeetingPanels } from '../legacy/panels-context';
import type { MeetingSlotProps } from '../types';

/** Whether `meetingId` is the meeting being recorded right now. */
function useIsLive(meetingId: MeetingId): boolean {
  const { meetingId: recording, phase } = useRecordingState();
  return recording === meetingId && (phase === 'recording' || phase === 'paused');
}

/** "Copy transcript" and the inherited tools. */
function TranscriptTools({ meetingId }: { meetingId: MeetingId }) {
  const share = useShareActions(meetingId);
  const legacy = useLegacyMeetingPanels();
  return (
    <>
      <IconButton
        icon={Copy}
        label={transcriptCopy.copyAll}
        onClick={() => {
          share.copy('transcript');
        }}
      />
      {legacy.status === 'ready' ? legacy.transcriptTools : null}
    </>
  );
}

/** The tab. */
export function TranscriptSlot({ meetingId, moment }: MeetingSlotProps) {
  const live = useIsLive(meetingId);
  return (
    <TranscriptPanel
      meetingId={meetingId}
      live={live}
      focusAt={moment?.at}
      onFocusDone={moment?.onShown}
      tools={<TranscriptTools meetingId={meetingId} />}
    />
  );
}
