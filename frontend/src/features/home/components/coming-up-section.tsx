/**
 * Home's Coming up (ORG-02): today and the next 14 days, each event with
 * Join, its record toggle and its menu. While sync is off it falls back to
 * the calendar feature's own `ComingUp`, which offers to connect one.
 */
import { useMemo } from 'react';

import {
  agendaDays,
  ComingUp,
  comingUpRange,
  type PlannedEvent,
  type RecordDecision,
  useCalendars,
  useCalendarStatus,
  useNow,
  useRecordPlan,
} from '@/features/calendar';
import { SkeletonText } from '@/ui/skeleton';

import { homeCopy } from '../copy';
import { SectionHeading } from './section-heading';
import { UpcomingRow } from './upcoming-row';

/** The plan's events, and each one's decision by ID. */
function split(plan: readonly PlannedEvent[]) {
  const decisions = new Map<string, RecordDecision>();
  for (const { event, decision } of plan) decisions.set(event.id, decision);
  return { events: plan.map(({ event }) => event), decisions };
}

/** The agenda with controls. */
function Agenda({ plan, now }: { plan: readonly PlannedEvent[]; now: Date }) {
  const calendars = useCalendars();
  const colors = useMemo(
    () => new Map((calendars.data ?? []).map((calendar) => [calendar.id, calendar.color ?? ''])),
    [calendars.data],
  );
  const { events, decisions } = split(plan);
  return (
    <div className="flex flex-col gap-2">
      {agendaDays(events, now).map((day) => (
        <section key={day.key} aria-label={day.label}>
          <h3 className="sticky top-0 z-(--z-sticky) flex h-8 items-center bg-bg px-3 text-xs font-semibold text-fg-muted">
            {day.label}
          </h3>
          <ul>
            {day.events.map((event) => {
              const decision = decisions.get(event.id);
              if (decision === undefined) return null;
              const color = event.calendarId === null ? '' : colors.get(event.calendarId);
              return (
                <UpcomingRow
                  key={event.id}
                  event={event}
                  decision={decision}
                  now={now}
                  color={color === '' ? undefined : color}
                />
              );
            })}
          </ul>
        </section>
      ))}
    </div>
  );
}

/** The plan once sync is on; empty or failing falls back to `ComingUp`. */
function PlannedComingUp() {
  const now = useNow();
  const plan = useRecordPlan(comingUpRange(now));
  if (plan.data === undefined && !plan.isError) return <SkeletonText lines={3} className="px-3" />;
  if (plan.data === undefined || plan.data.length === 0) return <ComingUp />;
  return (
    <section aria-labelledby="home-coming-up" className="flex flex-col gap-1">
      <SectionHeading id="home-coming-up">{homeCopy.comingUp}</SectionHeading>
      <Agenda plan={plan.data} now={now} />
    </section>
  );
}

/** Coming up with controls, or the prompt to connect a calendar. */
export function ComingUpSection() {
  const status = useCalendarStatus();
  if (status.data?.syncEnabled === true) return <PlannedComingUp />;
  return <ComingUp />;
}
