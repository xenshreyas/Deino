import type { ReactNode } from 'react';
import { TranscriptPanel } from '@/components/MeetingDetails/TranscriptPanel';
import { LegacyMeetingPanelsProvider } from './panels-context';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';
import { useShareActions } from '@/features/export';
import { asMeetingId } from '@/features/meetings';

/** The inherited transcript panel, published to the meeting page's Transcript slot. */
export default function PageContent({
  meeting,
  onRefetchTranscripts,
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
  children,
}: {
  /** The meeting page whose Transcript slot shows this panel. */
  children: ReactNode;
  meeting: any;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
  segments?: any[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;
}) {
  // EXP-01: copies go through the export plugin (the stored transcript).
  const share = useShareActions(asMeetingId(meeting.id));
  const meetingOperations = useMeetingOperations({ meeting });

  return (
    <LegacyMeetingPanelsProvider
      value={{
        status: 'ready',
        transcript: (
          <TranscriptPanel
            transcripts={meeting.transcripts}
            onCopyTranscript={() => share.copy('transcript')}
            onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
            isRecording={false}
            disableAutoScroll={true}
            usePagination={true}
            segments={segments}
            hasMore={hasMore}
            isLoadingMore={isLoadingMore}
            totalCount={totalCount}
            loadedCount={loadedCount}
            onLoadMore={onLoadMore}
            meetingId={meeting.id}
            meetingFolderPath={meeting.folder_path}
            onRefetchTranscripts={onRefetchTranscripts}
          />
        ),
      }}
    >
      {children}
    </LegacyMeetingPanelsProvider>
  );
}
