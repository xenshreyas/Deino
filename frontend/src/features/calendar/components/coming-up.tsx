/**
 * "Coming up": the user's next two weeks from their calendars (ui.md §4.1).
 * A simple list ORG-02 places on Home with Join and record controls; until
 * then it sits on the current Home while nothing is being recorded.
 */
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';

import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { Calendar } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { agendaDays, comingUpRange } from '../agenda';
import { calendarCopy } from '../copy';
import { useNow } from '../hooks/use-now';
import { useCalendarChanges, useCalendars, useCalendarStatus, useUpcoming } from '../queries';
import type { CalendarView } from '../types';
import { ComingUpList } from './coming-up-list';

/** Each calendar's colour by ID, for the rows' dots. */
function colorsOf(calendars: readonly CalendarView[] | undefined): ReadonlyMap<string, string> {
  const colors = new Map<string, string>();
  for (const calendar of calendars ?? []) {
    if (calendar.color !== null) colors.set(calendar.id, calendar.color);
  }
  return colors;
}

/** Sync is off: one calm prompt to set it up. */
function ConnectCalendar() {
  const navigate = useNavigate();
  return (
    <EmptyState
      className="pt-6"
      icon={Calendar}
      title={calendarCopy.connectTitle}
      description={calendarCopy.connectBody}
      action={
        <Button
          variant="secondary"
          onClick={() => void navigate({ to: '/settings', search: { section: 'calendar' } })}
        >
          {calendarCopy.connectAction}
        </Button>
      }
    />
  );
}

/** The agenda itself, once sync is on. */
function Agenda() {
  const now = useNow();
  const upcoming = useUpcoming(comingUpRange(now));
  const calendars = useCalendars();
  const colors = useMemo(() => colorsOf(calendars.data), [calendars.data]);
  if (upcoming.isError)
    return <p className="px-3 text-sm text-fg-muted">{calendarCopy.loadFailed}</p>;
  if (upcoming.data === undefined) return <SkeletonText lines={3} className="px-3" />;
  const days = agendaDays(upcoming.data, now);
  if (days.length === 0) {
    return (
      <EmptyState
        className="pt-6"
        icon={Calendar}
        title={calendarCopy.nothingComingUp}
        description={calendarCopy.nothingComingUpBody}
      />
    );
  }
  return <ComingUpList days={days} now={now} colors={colors} />;
}

/** The agenda while sync is on, else the prompt to connect a calendar. */
function ComingUpBody() {
  const status = useCalendarStatus();
  if (status.isError)
    return <p className="px-3 text-sm text-fg-muted">{calendarCopy.loadFailed}</p>;
  if (status.data === undefined) return <SkeletonText lines={3} className="px-3" />;
  return status.data.syncEnabled ? <Agenda /> : <ConnectCalendar />;
}

/** "Coming up", or the prompt to connect a calendar. */
export function ComingUp() {
  useCalendarChanges();
  return (
    <section aria-labelledby="coming-up" className="flex flex-col gap-1">
      <h2
        id="coming-up"
        className="px-3 text-xs font-semibold tracking-wide text-fg-muted uppercase"
      >
        {calendarCopy.comingUp}
      </h2>
      <ComingUpBody />
    </section>
  );
}
