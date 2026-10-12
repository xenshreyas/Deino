import type { ReactNode } from 'react';
import { LegacyMeetingPanelsProvider } from './panels-context';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';

/** The meeting's recording folder and its actions, published to the ⋯ menu (UX-05). */
export default function PageContent({
  meeting,
  onRefetchTranscripts,
  children,
}: {
  /** The meeting page whose ⋯ menu offers these actions. */
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
        recording: {
          folderPath: meeting.folder_path ?? null,
          openFolder: meetingOperations.handleOpenMeetingFolder,
          refetchTranscripts: onRefetchTranscripts ?? (async () => undefined),
        },
      }}
    >
      {children}
    </LegacyMeetingPanelsProvider>
  );
}
