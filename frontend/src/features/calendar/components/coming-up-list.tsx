/**
 * The "Coming up" agenda (ui.md §4.1): day headers, then one 40px row per
 * event with its time, calendar colour, title, people, call service and a
 * countdown. Presentational: events and the time come in as props.
 */
import { cn } from '@/ui/cn';

import { type AgendaDay, eventTime, startsIn } from '../agenda';
import { calendarCopy } from '../copy';
import type { CalendarEvent } from '../types';

/** Props of {@link ComingUpList}. */
export interface ComingUpListProps {
  days: readonly AgendaDay[];
  now: Date;
  /** Each calendar's colour, `#RRGGBB`, by calendar ID. */
  colors: ReadonlyMap<string, string>;
}

/** What a row says after its title: people, external, the call service. */
function eventMeta(event: CalendarEvent): string {
  const parts: string[] = [];
  if (event.attendees.length > 1) parts.push(calendarCopy.people(event.attendees.length));
  if (event.isExternal) parts.push(calendarCopy.external);
  if (event.platform !== null) parts.push(calendarCopy.platforms[event.platform]);
  return parts.join(' · ');
}

/** One event. */
function EventRow({
  event,
  now,
  color,
}: {
  event: CalendarEvent;
  now: Date;
  color: string | undefined;
}) {
  const countdown = startsIn(event, now);
  const live = countdown === calendarCopy.now;
  return (
    <li className="flex h-10 items-center gap-3 rounded-md px-3 hover:bg-bg-muted">
      <span className="w-14 shrink-0 text-xs text-fg-muted tabular-nums">{eventTime(event)}</span>
      <span
        aria-hidden
        className="size-2 shrink-0 rounded-full bg-fg-subtle"
        style={color === undefined ? undefined : { backgroundColor: color }}
      />
      <span className="min-w-0 flex-1 truncate text-sm text-fg">
        {event.title === '' ? calendarCopy.untitled : event.title}
      </span>
      <span className="hidden shrink-0 truncate text-xs text-fg-muted sm:inline">
        {eventMeta(event)}
      </span>
      {countdown === null ? null : (
        <span
          className={cn(
            'w-12 shrink-0 text-right text-xs tabular-nums',
            live ? 'text-accent' : 'text-fg-muted',
          )}
        >
          {countdown}
        </span>
      )}
    </li>
  );
}

/** The agenda, one group per day. */
export function ComingUpList({ days, now, colors }: ComingUpListProps) {
  return (
    <div className="flex flex-col gap-2">
      {days.map((day) => (
        <section key={day.key} aria-label={day.label}>
          <h3 className="sticky top-0 z-(--z-sticky) flex h-8 items-center bg-bg px-3 text-xs font-semibold text-fg-muted">
            {day.label}
          </h3>
          <ul>
            {day.events.map((event) => (
              <EventRow
                key={event.id}
                event={event}
                now={now}
                color={event.calendarId === null ? undefined : colors.get(event.calendarId)}
              />
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
