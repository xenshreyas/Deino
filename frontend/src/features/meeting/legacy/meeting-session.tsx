import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { useState, useEffect } from 'react';
import type { Transcript } from '@/types';
import type { ReactNode } from 'react';
import PageContent from './meeting-panels';
import { LegacyMeetingPanelsProvider } from './panels-context';
import type { MeetingSource } from '../search';
import { usePaginatedTranscripts } from '@/hooks/usePaginatedTranscripts';

interface MeetingDetailsResponse {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  transcripts: Transcript[];
  folder_path?: string;
}

interface LegacyMeetingSessionProps {
  meetingId: string;
  /** How the user got here; kept for the route contract. */
  source?: MeetingSource | undefined;
  /** The meeting page; its Transcript slot reads the published panel. */
  children: ReactNode;
}

/**
 * One meeting's inherited transcript session: loads its transcript pages and
 * publishes the transcript panel to the meeting page's Transcript slot
 * (`panels-context.tsx`). The notes moved to typed queries (NOT-01, NOT-02);
 * TRN-01 replaces what is left.
 */
export function LegacyMeetingSession({ meetingId, children }: LegacyMeetingSessionProps) {
  const { setCurrentMeeting, refetchMeetings } = useSidebar();
  const [meetingDetails, setMeetingDetails] = useState<MeetingDetailsResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const {
    metadata,
    segments,
    transcripts,
    isLoading: isLoadingTranscripts,
    isLoadingMore,
    hasMore,
    totalCount,
    loadedCount,
    loadMore,
    refetch,
    error: transcriptError,
  } = usePaginatedTranscripts({ meetingId: meetingId || '' });

  // Sync meeting metadata from pagination hook to meeting details state
  useEffect(() => {
    if (
      isLoadingTranscripts ||
      !meetingId ||
      meetingId === 'intro-call' ||
      metadata?.id !== meetingId
    ) {
      // Keep the current view until the selected meeting's first transcript page is ready.
      return;
    }
    if (metadata) {
      setMeetingDetails({
        id: metadata.id,
        title: metadata.title,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        transcripts,
        folder_path: metadata.folder_path,
      });
      setCurrentMeeting({ id: metadata.id, title: metadata.title });
    }
  }, [metadata, transcripts, meetingId, isLoadingTranscripts, setCurrentMeeting]);

  // Handle transcript loading errors
  useEffect(() => {
    if (transcriptError) {
      console.error('Error loading transcripts:', transcriptError);
      setError(transcriptError);
    }
  }, [transcriptError]);

  // Reset state when meetingId changes (prevent race conditions)
  useEffect(() => {
    setMeetingDetails(null);
    setError(!meetingId || meetingId === 'intro-call' ? 'No meeting selected' : null);
  }, [meetingId]);

  if (error) {
    return (
      <LegacyMeetingPanelsProvider value={{ status: 'error', message: error }}>
        {children}
      </LegacyMeetingPanelsProvider>
    );
  }

  // Keep the page and its header while the selected meeting's data loads.
  if (!meetingDetails || meetingDetails.id !== meetingId) {
    return (
      <LegacyMeetingPanelsProvider value={{ status: 'loading' }}>
        {children}
      </LegacyMeetingPanelsProvider>
    );
  }

  return (
    <PageContent
      key={meetingId}
      meeting={meetingDetails}
      onMeetingUpdated={async () => {
        await refetchMeetings();
      }}
      onRefetchTranscripts={refetch}
      segments={segments}
      hasMore={hasMore}
      isLoadingMore={isLoadingMore}
      totalCount={totalCount}
      loadedCount={loadedCount}
      onLoadMore={loadMore}
    >
      {children}
    </PageContent>
  );
}
