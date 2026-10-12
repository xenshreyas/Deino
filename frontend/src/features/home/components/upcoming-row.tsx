/**
 * One Coming up row (ui.md §4.1): time, calendar colour, title, people,
 * then the countdown, Join, the record toggle and the ⋯ menu, which a
 * right-click anywhere on the row opens too. Join opens the call and
 * starts recording in one Rust call (ORG-02).
 */
import { useState } from 'react';
import { toast } from 'sonner';

import {
  type CalendarEvent,
  eventTime,
  type RecordDecision,
  startsIn,
  useJoin,
} from '@/features/calendar';
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { Hint } from '@/ui/tooltip';

import { homeCopy } from '../copy';
import { EventMenu } from './event-menu';
import { RecordToggle } from './record-toggle';

/** Props of {@link UpcomingRow}. */
export interface UpcomingRowProps {
  event: CalendarEvent;
  decision: RecordDecision;
  now: Date;
  /** The calendar's colour, `#RRGGBB`. */
  color: string | undefined;
}

/** Join & record; only for events with a call link. */
function JoinButton({ event }: { event: CalendarEvent }) {
  const join = useJoin();
  if (event.conferenceUrl === null) return null;
  return (
    <Hint label={homeCopy.joinTitle}>
      <Button
        variant="secondary"
        size="sm"
        disabled={join.isPending}
        onClick={() => {
          join.mutate(
            { id: event.id, mode: 'joinAndRecord' },
            { onError: () => toast.error(homeCopy.joinFailed) },
          );
        }}
      >
        {homeCopy.join}
      </Button>
    </Hint>
  );
}

/** The time until it starts, or "Now" in the accent while it runs. */
function Countdown({ event, now }: { event: CalendarEvent; now: Date }) {
  const countdown = startsIn(event, now);
  if (countdown === null) return null;
  const live = Date.parse(event.startAt) <= now.getTime();
  return (
    <span
      className={cn(
        'w-12 shrink-0 text-right text-xs tabular-nums',
        live ? 'text-accent' : 'text-fg-muted',
      )}
    >
      {countdown}
    </span>
  );
}

/** One upcoming event with its controls. */
export function UpcomingRow({ event, decision, now, color }: UpcomingRowProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  return (
    <li
      className="group flex h-10 items-center gap-3 rounded-md px-3 hover:bg-bg-muted"
      onContextMenu={(click) => {
        click.preventDefault();
        setMenuOpen(true);
      }}
    >
      <span className="w-14 shrink-0 text-xs text-fg-muted tabular-nums">{eventTime(event)}</span>
      <span
        aria-hidden
        className="size-2 shrink-0 rounded-full bg-fg-subtle"
        style={color === undefined ? undefined : { backgroundColor: color }}
      />
      <span className="min-w-0 flex-1 truncate text-sm text-fg">
        {event.title === '' ? 'Untitled event' : event.title}
      </span>
      <Countdown event={event} now={now} />
      <JoinButton event={event} />
      <RecordToggle event={event} decision={decision} />
      <EventMenu event={event} open={menuOpen} onOpenChange={setMenuOpen} />
    </li>
  );
}
