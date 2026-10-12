/**
 * The meeting reminder card (ORG-02, presentational): "starts in a
 * minute", the title, time and people, then Join & record (with a call
 * link), Record only, and dismiss. Props in, events out; `MeetingReminder`
 * wires it to Rust.
 *
 * It floats in its own transparent, non-activating window, so it carries
 * the floating-layer recipe of CAP-06's prompt (raised surface, r12,
 * popover shadow, entering from the top edge).
 */
import { differenceInMinutes, format } from 'date-fns';

import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { Calendar, ICON_PROPS, X } from '@/ui/icons';

import { calendarCopy } from '../copy';
import { rulesCopy } from '../rules-copy';
import type { JoinMode } from '../plan';
import type { CalendarEvent } from '../types';

const copy = rulesCopy.reminder;

/** How many invitees are named before "+N". */
const NAMED_PEOPLE = 3;

/** Props of {@link ReminderCard}. */
export interface ReminderCardProps {
  event: CalendarEvent;
  now: Date;
  /** A button's call is on its way; the buttons wait. */
  busy: boolean;
  /** The last call failed. */
  failed: boolean;
  onJoin: (mode: JoinMode) => void;
  onDismiss: () => void;
}

/** "Starts in a minute", or "Starting now" once it has. */
export function reminderLead(event: CalendarEvent, now: Date): string {
  const start = new Date(event.startAt);
  if (start <= now) return copy.started;
  return copy.startsIn(differenceInMinutes(start, now, { roundingMethod: 'ceil' }));
}

/** The first invitees by name (else address), then "+N". */
export function peopleLine(event: CalendarEvent): string {
  const names = event.attendees
    .map((attendee) => attendee.name ?? attendee.email ?? '')
    .filter((name) => name !== '');
  const shown = names.slice(0, NAMED_PEOPLE).join(', ');
  const rest = names.length - NAMED_PEOPLE;
  return rest > 0 ? `${shown} ${copy.others(rest)}` : shown;
}

function Heading({ event, now }: { event: CalendarEvent; now: Date }) {
  const title = event.title === '' ? calendarCopy.untitled : event.title;
  const time = `${format(new Date(event.startAt), 'HH:mm')}–${format(new Date(event.endAt), 'HH:mm')}`;
  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <p className="text-xs font-medium text-accent">{reminderLead(event, now)}</p>
      <h1 className="truncate text-sm font-semibold text-fg" title={title}>
        {title}
      </h1>
      <p className="truncate text-xs text-fg-muted">
        <span className="tabular-nums">{time}</span>
        {` · ${peopleLine(event)}`}
      </p>
    </div>
  );
}

function Actions({ props }: { props: ReminderCardProps }) {
  const { busy, event, onJoin } = props;
  return (
    <footer className="flex items-center justify-end gap-1 border-t border-border px-2 py-2">
      <Button
        variant={event.conferenceUrl === null ? 'primary' : 'ghost'}
        size="sm"
        disabled={busy}
        onClick={() => {
          onJoin('recordOnly');
        }}
      >
        {copy.recordOnly}
      </Button>
      {event.conferenceUrl === null ? null : (
        <Button
          variant="primary"
          size="sm"
          disabled={busy}
          onClick={() => {
            onJoin('joinAndRecord');
          }}
        >
          {copy.joinAndRecord}
        </Button>
      )}
    </footer>
  );
}

/** The reminder card. */
export function ReminderCard(props: ReminderCardProps) {
  return (
    <section
      aria-label={copy.label}
      className={cn(
        'relative flex flex-col overflow-hidden rounded-lg bg-surface-raised text-fg shadow-popover',
        'animate-in duration-(--dur-slow) ease-out fade-in-0 slide-in-from-top-1',
      )}
    >
      <div className="flex items-start gap-3 px-4 pt-3.5 pb-3">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-accent-soft text-accent">
          <Calendar {...ICON_PROPS} />
        </span>
        <Heading event={props.event} now={props.now} />
        <Button
          variant="ghost"
          size="icon"
          className="-mt-1 -mr-2"
          aria-label={copy.dismiss}
          disabled={props.busy}
          onClick={props.onDismiss}
        >
          <X {...ICON_PROPS} />
        </Button>
      </div>
      {props.failed ? (
        <p role="alert" className="px-4 pb-2 text-xs text-danger">
          {copy.failed}
        </p>
      ) : null}
      <Actions props={props} />
    </section>
  );
}
