import { format, isSameDay, subDays } from 'date-fns';

import { type MeetingId, useMeeting } from '@/features/meetings';
import { PipelineStrip } from '@/features/pipeline';
import { Chip, MetaChip } from '@/ui/chip';
import { Calendar, Plus, Users } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';
import { Hint } from '@/ui/tooltip';

import { meetingCopy } from '../copy';
import { MeetingTitle } from './meeting-title';

/** `Today 14:00`, `Yesterday 09:30` or `Oct 8, 14:00`. */
export function meetingDateLabel(iso: string, now: Date): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  if (isSameDay(date, now)) return `Today ${format(date, 'HH:mm')}`;
  if (isSameDay(date, subDays(now, 1))) return `Yesterday ${format(date, 'HH:mm')}`;
  return format(date, 'MMM d, HH:mm');
}

/** A placeholder chip for data a later card fills (attendees, tags). */
function SoonChip({ label, icon }: { label: string; icon: typeof Plus }) {
  return (
    <Hint label={meetingCopy.comingSoon}>
      <Chip
        variant="ghost"
        icon={icon}
        aria-disabled
        className="opacity-0 group-hover/header:opacity-100 focus-visible:opacity-100"
      >
        {label}
      </Chip>
    </Hint>
  );
}

/**
 * Title, then a row of meta chips: date, attendees and tags (placeholders
 * until ORG-04 and ORG-06 provide them), then the processing status strip.
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
        <SoonChip icon={Users} label={meetingCopy.addAttendees} />
        <SoonChip icon={Plus} label={meetingCopy.addTag} />
      </div>
      {/* FND-08: processing status, minimally placed; FND-10 follow-up gives it its home. */}
      <PipelineStrip meetingId={meetingId} />
    </header>
  );
}
