/**
 * "Transcript deleted" (PRV-01): what the Transcript tab shows once the
 * retention sweep deleted the meeting's transcript. The notes stay; this
 * says so, and when.
 */
import type { ReactNode } from 'react';

import { ICON_PROPS_LARGE, ScrollText } from '@/ui/icons';

import { retentionCopy } from '../copy';
import { useMeetingRetention } from '../queries';
import type { MeetingId } from '../types';

const copy = retentionCopy.deleted;

/** The notice, for a transcript deleted at `deletedAt`. Presentational. */
export function TranscriptDeleted({ deletedAt }: { deletedAt: string }) {
  return (
    <div
      role="status"
      className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center"
    >
      <ScrollText {...ICON_PROPS_LARGE} className="text-fg-subtle" />
      <p className="text-sm font-medium text-fg">{copy.transcriptTitle}</p>
      <p className="max-w-sm text-xs text-pretty text-fg-muted">{copy.transcriptBody(deletedAt)}</p>
    </div>
  );
}

/** Props of {@link TranscriptGate}. */
export interface TranscriptGateProps {
  meetingId: MeetingId;
  /** The transcript, while it is kept. */
  children: ReactNode;
}

/** The transcript, or "Transcript deleted" once retention deleted it. */
export function TranscriptGate({ meetingId, children }: TranscriptGateProps) {
  const deletedAt = useMeetingRetention(meetingId).data?.transcriptDeletedAt ?? null;
  return deletedAt === null ? children : <TranscriptDeleted deletedAt={deletedAt} />;
}
