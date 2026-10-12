import { format, isSameDay, subDays } from 'date-fns';

import { OpenNoteReporter } from '@/features/calendar';
import { MeetingAttendees } from '@/features/entities';
import { MeetingFolders } from '@/features/folders';
import { MeetingFollowUp } from '@/features/followups';
import { type MeetingId, useMeeting } from '@/features/meetings';
import { PipelineStatus } from '@/features/pipeline';
import { ResumeRecordingButton } from '@/features/recording';
import { MeetingTags } from '@/features/tags';
import { MetaChip } from '@/ui/chip';
import { Calendar } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { MeetingMenu } from './meeting-menu';
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
 * Title and the ⋯ menu (UX-05), then a row of meta chips (UX-08): the date,
 * attendees (ORG-04), folders, tags only once there are some (their "Add"
 * ghosts wait for hover), and "Follow-up email ready" while a draft waits
 * (NOT-07); Resume for a meeting that ended in the last two hours. Below,
 * one processing line, only while work runs or after a failure (UX-01).
 * The calendar event and related meetings are in Details.
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
      <div className="flex items-start gap-2">
        <div className="min-w-0 flex-1">
          <MeetingTitle meeting={meeting.data} />
        </div>
        <MeetingMenu meetingId={meetingId} />
      </div>
      <div className="flex flex-wrap items-center gap-1.5">
        <MetaChip icon={Calendar}>{meetingDateLabel(meeting.data.createdAt, new Date())}</MetaChip>
        <MeetingAttendees meetingId={meetingId} restingClassName={REVEAL_ON_HOVER} />
        <MeetingFolders meetingId={meetingId} addChipClassName={REVEAL_ON_HOVER} />
        <MeetingTags meetingId={meetingId} addChipClassName={REVEAL_ON_HOVER} />
        <MeetingFollowUp meetingId={meetingId} />
        <span className="ml-auto">
          <ResumeRecordingButton meetingId={meetingId} />
        </span>
      </div>
      {/* ORG-02: tells Rust this event's note is open (the start-time auto-record trigger). */}
      <OpenNoteReporter meetingId={meetingId} />
      <PipelineStatus meetingId={meetingId} />
    </header>
  );
}
