/**
 * The Transcript tab (TRN-01): the meeting's transcript view, live while
 * this meeting is being recorded, opened at a linked moment (`?t=`) or
 * line (`?segment=`). Its
 * tools are "Copy transcript" and, while the inherited session still owns
 * them, the recording folder and re-transcription buttons. Selected words
 * can be commented on (TRN-06), clipped or linked to (TRN-04).
 */
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';

import { ClipDialog } from '@/features/clips';
import { CommentLayer } from '@/features/comments';
import { mediaQuery } from '@/features/player';
import type { MeetingId } from '@/features/meetings';
import { useShareActions } from '@/features/export';
import { useRecordingState } from '@/features/recording';
import { TranscriptPanel, transcriptCopy, transcriptQuery } from '@/features/transcript';
import { Copy } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { useLegacyMeetingPanels } from '../legacy/panels-context';
import type { MeetingSlotProps } from '../types';
import { selectionActions, selectionSpan } from './transcript-selection';

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

/** Clip and Copy link for a selected range of the transcript. */
function useSelectionActions(meetingId: MeetingId) {
  const { data: transcript } = useQuery(transcriptQuery(meetingId));
  const { data: media } = useQuery(mediaQuery(meetingId));
  const playable = (media?.files.length ?? 0) > 0;
  const segments = useMemo(
    () => new Map((transcript?.segments ?? []).map((segment) => [segment.id, segment])),
    [transcript],
  );
  return (range: Range) =>
    selectionActions(meetingId, selectionSpan(document.body, range, segments), playable);
}

/** The tab. */
export function TranscriptSlot({ meetingId, moment }: MeetingSlotProps) {
  const live = useIsLive(meetingId);
  const actions = useSelectionActions(meetingId);
  return (
    <CommentLayer meetingId={meetingId} target="transcript" actions={actions} className="h-full">
      <TranscriptPanel
        meetingId={meetingId}
        live={live}
        moment={moment?.target}
        onMomentShown={moment?.onShown}
        tools={<TranscriptTools meetingId={meetingId} />}
      />
      <ClipDialog />
    </CommentLayer>
  );
}
