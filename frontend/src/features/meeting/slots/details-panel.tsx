/**
 * The Details panel (UX-04): who spoke (CAP-04), the meeting's comments
 * (TRN-06) and clips (TRN-04) when it has any, its calendar event and
 * related meetings (ORG-03), and last, collapsed, what the vocabulary
 * corrected in its transcript (TRN-05). The header already shows the date,
 * so the panel has no created or edited facts.
 */
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';

import { MeetingClips } from '@/features/clips';
import { CommentsPanel } from '@/features/comments';
import { MeetingLinksPanel } from '@/features/event-links';
import type { MeetingId } from '@/features/meetings';
import { ParticipantsPanel } from '@/features/speakers';
import { transcriptQuery } from '@/features/transcript';
import { MeetingCorrections } from '@/features/vocabulary';
import { PanelSection } from '@/ui/panel-section';

import { meetingCopy } from '../copy';
import type { MeetingSlotProps } from '../types';

/** The comments, each transcript one checked against the transcript's current lines. */
function Comments({ meetingId }: { meetingId: MeetingId }) {
  const { data } = useQuery(transcriptQuery(meetingId));
  const lineTexts = useMemo(
    () => (data === undefined ? null : new Map(data.segments.map((line) => [line.id, line.text]))),
    [data],
  );
  return <CommentsPanel meetingId={meetingId} lineTexts={lineTexts} />;
}

/** The panel. */
export function DetailsPanelSlot({ meetingId }: MeetingSlotProps) {
  return (
    <div className="flex flex-col gap-6 px-4 py-4">
      <PanelSection title={meetingCopy.participants}>
        <ParticipantsPanel meetingId={meetingId} />
      </PanelSection>
      <Comments meetingId={meetingId} />
      <MeetingClips meetingId={meetingId} />
      <MeetingLinksPanel meetingId={meetingId} />
      <MeetingCorrections meetingId={meetingId} />
    </div>
  );
}
