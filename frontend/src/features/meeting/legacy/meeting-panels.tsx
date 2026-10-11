import type { ReactNode } from 'react';
import { TranscriptButtonGroup } from '@/components/MeetingDetails/TranscriptButtonGroup';
import { LegacyMeetingPanelsProvider } from './panels-context';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';

/** The Transcript tab's inherited tools, published to the meeting page (TRN-01 shows them). */
export default function PageContent({
  meeting,
  onRefetchTranscripts,
  children,
}: {
  /** The meeting page whose Transcript tab shows these tools. */
  children: ReactNode;
  meeting: any;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
}) {
  const meetingOperations = useMeetingOperations({ meeting });

  return (
    <LegacyMeetingPanelsProvider
      value={{
        status: 'ready',
        transcriptTools: (
          <TranscriptButtonGroup
            onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
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
