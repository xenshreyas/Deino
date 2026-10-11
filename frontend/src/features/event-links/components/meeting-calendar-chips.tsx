/**
 * The meeting header's calendar chips: the linked event (and its picker),
 * then "Previous" when an earlier meeting of the same series exists
 * (Granola §6.5), a link to it.
 */
import { Link } from '@tanstack/react-router';

import { chipLinkClassName } from '@/ui/chip';
import { History, ICON_PROPS } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { eventLinksCopy } from '../copy';
import { dayLabel } from '../format';
import { useRelatedMeetings } from '../queries';
import type { MeetingId } from '../types';
import { MeetingEventChip, type MeetingEventChipProps } from './meeting-event-chip';

/** "Previous · Oct 2", opening the meeting before this one in its series. */
export function PreviousMeetingChip({ meetingId }: { meetingId: MeetingId }) {
  const related = useRelatedMeetings(meetingId);
  const previous = related.data?.previous;
  if (previous === undefined || previous === null) return null;
  return (
    <Hint label={`${eventLinksCopy.previousInSeries}: ${previous.title}`}>
      <Link
        to="/meetings/$meetingId"
        params={{ meetingId: previous.id }}
        aria-label={eventLinksCopy.openPrevious(previous.title)}
        className={chipLinkClassName}
      >
        <History {...ICON_PROPS} size={14} />
        {eventLinksCopy.previousChip}
        <span className="text-fg-subtle tabular-nums">
          {dayLabel(previous.happenedAt, new Date())}
        </span>
      </Link>
    </Hint>
  );
}

/** The event chip, then the previous meeting of the series. */
export function MeetingCalendarChips(props: MeetingEventChipProps) {
  return (
    <>
      <MeetingEventChip {...props} />
      <PreviousMeetingChip meetingId={props.meetingId} />
    </>
  );
}
