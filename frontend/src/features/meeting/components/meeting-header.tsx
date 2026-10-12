import { format, isSameDay, subDays } from 'date-fns';

import { MeetingCalendarChips } from '@/features/event-links';
import { MeetingAttendees } from '@/features/entities';
import { MeetingFolders } from '@/features/folders';
import { type MeetingId, useMeeting } from '@/features/meetings';
import { PipelineStrip } from '@/features/pipeline';
import { MeetingTags } from '@/features/tags';
import { MetaChip } from '@/ui/chip';
import { Calendar } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { MeetingTitle } from './meeting-title';

/** `Today 14:00`, `Yesterday 09:30` or `Oct 8, 14:00`. */
export function meetingDateLabel(iso: string, now: Date): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  if (isSameDay(date, now)) return `Today ${format(date, 'HH:mm')}`;
  if (isSameDay(date, subDays(now, 1))) return `Yesterday ${format(date, 'HH:mm')}`;
  return format(date, 'MMM d, HH:mm');
}

/** Ghost "Add …" chips rest invisible until the header is hovered or focused (ui.md §3 Meeting header). */
const REVEAL_ON_HOVER =
  'opacity-0 group-hover/header:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100';

/**
 * Title, then a row of meta chips: date, the calendar event and the
 * previous meeting of its series (ORG-03), attendees (ORG-04), folders
 * and tags, then the processing status strip.
 */
export function MeetingHeader({ meetingId }: { meetingId: MeetingId }) {
  const meeting = useMeeting(meetingId);
  if (meeting.data === undefined) {
    return (
      <header className="flex flex-col gap-3 pt-8" aria-busy>
        <Skeleton className="h-6 w-2/5" />
        <Skeleton className="h-5 w-32 rounded-full" />
      </header>
    );
  }
  return (
    <header className="group/header flex flex-col gap-2 pt-8">
      <MeetingTitle meeting={meeting.data} />
      <div className="flex flex-wrap items-center gap-1.5">
        <MetaChip icon={Calendar}>{meetingDateLabel(meeting.data.createdAt, new Date())}</MetaChip>
        <MeetingCalendarChips meetingId={meetingId} addChipClassName={REVEAL_ON_HOVER} />
        <MeetingAttendees meetingId={meetingId} restingClassName={REVEAL_ON_HOVER} />
        <MeetingFolders meetingId={meetingId} addChipClassName={REVEAL_ON_HOVER} />
        <MeetingTags meetingId={meetingId} addChipClassName={REVEAL_ON_HOVER} />
      </div>
      {/* FND-08: processing status, minimally placed; FND-10 follow-up gives it its home. */}
      <PipelineStrip meetingId={meetingId} />
    </header>
  );
}
