/**
 * The Clips tab (TRN-04) and the Comments panel (TRN-06). The panel marks
 * a transcript comment "Detached" against the transcript's current lines.
 */
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';

import { ClipsTab } from '@/features/clips';
import { CommentsPanel } from '@/features/comments';
import { transcriptQuery } from '@/features/transcript';

import type { MeetingSlotProps } from '../types';

/** Clips tab. */
export function ClipsSlot({ meetingId }: MeetingSlotProps) {
  return <ClipsTab meetingId={meetingId} />;
}

/** Comments panel. */
export function CommentsPanelSlot({ meetingId }: MeetingSlotProps) {
  const { data } = useQuery(transcriptQuery(meetingId));
  const lineTexts = useMemo(
    () => (data === undefined ? null : new Map(data.segments.map((line) => [line.id, line.text]))),
    [data],
  );
  return <CommentsPanel meetingId={meetingId} lineTexts={lineTexts} />;
}
