import { format } from 'date-fns';

import { MeetingLinksPanel } from '@/features/event-links';
import { useMeeting } from '@/features/meetings';
import { MeetingCorrections } from '@/features/vocabulary';
import { Skeleton } from '@/ui/skeleton';

import { meetingCopy } from '../copy';
import type { MeetingSlotProps } from '../types';

/** `Oct 8, 2026 at 14:00`, or the raw text when it is not a date. */
function longDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : format(date, "MMM d, yyyy 'at' HH:mm");
}

/** One label and value pair. */
function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <dt className="text-xs text-fg-muted">{label}</dt>
      <dd className="text-sm text-fg tabular-nums">{value}</dd>
    </div>
  );
}

/**
 * Details panel: when the meeting was saved and last edited, its calendar
 * event and related meetings (ORG-03), and what the vocabulary corrected in
 * its transcript (TRN-05).
 */
export function DetailsPanelSlot({ meetingId }: MeetingSlotProps) {
  const meeting = useMeeting(meetingId);
  if (meeting.data === undefined) {
    return (
      <div className="flex flex-col gap-3 px-4 py-5">
        <Skeleton className="h-2.5 w-1/3" />
        <Skeleton className="h-2.5 w-2/3" />
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-6 px-4 py-5">
      <dl className="flex flex-col gap-4">
        <Fact label={meetingCopy.created} value={longDate(meeting.data.createdAt)} />
        <Fact label={meetingCopy.updated} value={longDate(meeting.data.updatedAt)} />
      </dl>
      <MeetingLinksPanel meetingId={meetingId} />
      <MeetingCorrections meetingId={meetingId} />
    </div>
  );
}
