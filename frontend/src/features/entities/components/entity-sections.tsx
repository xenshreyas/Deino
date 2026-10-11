/**
 * The sections of a person or company page: a quiet 12/600 label over its
 * rows (ui.md §4.5), and the rows themselves: meetings, open action items
 * and upcoming events. Each section says plainly when it is empty.
 */
import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';

import { cn } from '@/ui/cn';
import { Calendar, Circle, ICON_PROPS, NotebookText, Video } from '@/ui/icons';

import { entitiesCopy } from '../copy';
import { dueLabel, duration, eventTime, relativeDay } from '../format';
import type { EntityActionItem, EntityMeeting, UpcomingEvent } from '../types';

/** Props of {@link Section}. */
export interface SectionProps {
  id: string;
  title: string;
  /** A small action at the right of the label. */
  trailing?: ReactNode;
  children: ReactNode;
}

/** A labelled block of the page. */
export function Section({ id, title, trailing, children }: SectionProps) {
  return (
    <section aria-labelledby={id} className="flex flex-col">
      <div className="flex h-8 items-center justify-between gap-3 px-3">
        <h2 id={id} className="text-xs font-semibold text-fg-muted">
          {title}
        </h2>
        {trailing}
      </div>
      {children}
    </section>
  );
}

/** A section's resting line. */
export function Quiet({ children }: { children: ReactNode }) {
  return <p className="px-3 py-2 text-sm text-fg-subtle">{children}</p>;
}

const ROW = 'flex h-10 items-center gap-3 rounded-md px-3 text-sm';

/** Meetings, most recent first, each opening the meeting. */
export function MeetingRows({ meetings, now }: { meetings: readonly EntityMeeting[]; now: Date }) {
  if (meetings.length === 0) return <Quiet>{entitiesCopy.noMeetings}</Quiet>;
  return (
    <ul className="flex flex-col">
      {meetings.map((meeting) => (
        <li key={meeting.id}>
          <Link
            to="/meetings/$meetingId"
            params={{ meetingId: meeting.id }}
            className={`${ROW} transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:bg-bg-muted`}
          >
            <NotebookText {...ICON_PROPS} className="shrink-0 text-fg-muted" />
            <span className="min-w-0 flex-1 truncate text-fg">{meeting.title}</span>
            <span className="shrink-0 text-xs text-fg-muted tabular-nums">
              {relativeDay(meeting.at, now)}
            </span>
            <span className="w-12 shrink-0 text-right text-xs text-fg-subtle tabular-nums">
              {duration(meeting.durationS)}
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}

/** When an item is due; a date that has passed reads as a warning. */
function DueDate({ iso, now }: { iso: string | null; now: Date }) {
  const due = iso === null ? null : dueLabel(iso, now);
  return (
    <span
      className={cn(
        'w-20 shrink-0 text-right text-xs tabular-nums',
        due?.overdue === true ? 'text-warning-text' : 'text-fg-muted',
      )}
    >
      {due === null ? '—' : due.text}
    </span>
  );
}

/** Open action items, soonest due first. */
export function ActionItemRows({
  items,
  now,
  owner,
}: {
  items: readonly EntityActionItem[];
  now: Date;
  /** Who each item is for, when the page lists several people's. */
  owner?: ((item: EntityActionItem) => string | null) | undefined;
}) {
  if (items.length === 0) return <Quiet>{entitiesCopy.noActionItems}</Quiet>;
  return (
    <ul className="flex flex-col">
      {items.map((item) => (
        <li key={item.id} className={ROW}>
          <Circle {...ICON_PROPS} className="shrink-0 text-fg-subtle" />
          <span className="min-w-0 flex-1 truncate text-fg">{item.title}</span>
          {owner === undefined ? null : (
            <span className="shrink-0 truncate text-xs text-fg-muted">{owner(item)}</span>
          )}
          <DueDate iso={item.dueAt} now={now} />
        </li>
      ))}
    </ul>
  );
}

/** Calendar events in the next two weeks. */
export function UpcomingRows({ events }: { events: readonly UpcomingEvent[] }) {
  if (events.length === 0) return <Quiet>{entitiesCopy.noUpcoming}</Quiet>;
  return (
    <ul className="flex flex-col">
      {events.map((event) => {
        const EventIcon = event.conferenceUrl === null ? Calendar : Video;
        return (
          <li key={event.id} className={ROW}>
            <EventIcon {...ICON_PROPS} className="shrink-0 text-fg-muted" />
            <span className="min-w-0 flex-1 truncate text-fg">{event.title}</span>
            <span className="shrink-0 text-xs text-fg-muted tabular-nums">
              {eventTime(event.startAt)}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
