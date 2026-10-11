/**
 * The Details panel's calendar sections: the linked event (when, guests,
 * description: what linking brought in), "Previous in series" and the
 * other related meetings, each a link to its page.
 */
import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';
import { History, ICON_PROPS } from '@/ui/icons';

import { eventLinksCopy } from '../copy';
import { dayLabel, eventTitle, eventWhen, guestNames } from '../format';
import { useLinkedEvent, useRelatedMeetings } from '../queries';
import type { CalendarEvent, MeetingId, RelatedMeeting } from '../types';

/** Guests listed by name before "and N more". */
const GUESTS_SHOWN = 6;

/** A titled section of the panel. */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section aria-label={title} className="flex flex-col gap-1.5">
      <h3 className="text-xs text-fg-muted">{title}</h3>
      {children}
    </section>
  );
}

/** "Ada, Bob, Maya and 2 more". */
function guestLine(event: CalendarEvent): string {
  const names = guestNames(event);
  const shown = names.slice(0, GUESTS_SHOWN).join(', ');
  const rest = names.length - GUESTS_SHOWN;
  return rest > 0 ? `${shown} ${eventLinksCopy.moreGuests(rest)}` : shown;
}

/** The linked event: title, when, guests and description. */
function EventSection({ event }: { event: CalendarEvent }) {
  const guests = guestLine(event);
  const description = event.description.trim();
  return (
    <Section title={eventLinksCopy.eventSection}>
      <p className="text-sm text-fg">{eventTitle(event)}</p>
      <p className="text-xs text-fg-muted tabular-nums">{eventWhen(event, new Date())}</p>
      {guests === '' ? null : (
        <p className="text-xs text-pretty text-fg-muted">
          <span className="sr-only">{eventLinksCopy.guests}: </span>
          {guests}
        </p>
      )}
      {description === '' ? null : (
        <p className="line-clamp-6 pt-1 text-sm whitespace-pre-line text-fg-muted">{description}</p>
      )}
    </Section>
  );
}

/** One related meeting: its title, and the day it took place. */
function RelatedRow({ meeting, icon }: { meeting: RelatedMeeting; icon?: boolean }) {
  return (
    <Link
      to="/meetings/$meetingId"
      params={{ meetingId: meeting.id }}
      title={eventLinksCopy.by[meeting.by]}
      className={cn(
        '-mx-2 flex h-8 items-center gap-2 rounded-md px-2 text-sm text-fg',
        'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted',
        'focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-focus-ring',
      )}
    >
      {icon === true ? <History {...ICON_PROPS} className="shrink-0 text-fg-muted" /> : null}
      <span className="min-w-0 flex-1 truncate">{meeting.title}</span>
      <span className="shrink-0 text-xs text-fg-subtle tabular-nums">
        {dayLabel(meeting.happenedAt, new Date())}
      </span>
    </Link>
  );
}

/** "Previous in series" and the related list; nothing when there are none. */
function RelatedSections({ meetingId }: { meetingId: MeetingId }) {
  const related = useRelatedMeetings(meetingId);
  if (related.data === undefined) return null;
  const { previous, others } = related.data;
  return (
    <>
      {previous === null ? null : (
        <Section title={eventLinksCopy.previousInSeries}>
          <RelatedRow meeting={previous} icon />
        </Section>
      )}
      {others.length === 0 ? null : (
        <Section title={eventLinksCopy.relatedSection}>
          <div className="flex flex-col">
            {others.map((meeting) => (
              <RelatedRow key={meeting.id} meeting={meeting} />
            ))}
          </div>
        </Section>
      )}
    </>
  );
}

/** The calendar sections of the Details panel. */
export function MeetingLinksPanel({ meetingId }: { meetingId: MeetingId }) {
  const linked = useLinkedEvent(meetingId);
  return (
    <>
      {linked.data === undefined || linked.data === null ? null : (
        <EventSection event={linked.data} />
      )}
      <RelatedSections meetingId={meetingId} />
    </>
  );
}
