/**
 * The meeting reminder card (ORG-02, presentational), design R of
 * `docs/mockups/meeting-prompt.html`, after Granola's: one row with a solid
 * bar, the title over "17:30 – 18:00", and one split button.
 * With a call link it is "Join meeting & open Deino" with the service's
 * logo; without, "Take notes" with the Deino mark. The caret holds "Take
 * notes without joining" (with a link) and Dismiss. Props in, events out;
 * `MeetingReminder` wires it to Rust.
 *
 * It floats in its own transparent, non-activating window that reaches the
 * screen's right edge, so it slides in from off-screen (`ui/edge-card`).
 */
import { differenceInMinutes, format } from 'date-fns';

import { cn } from '@/ui/cn';
import { DrainBar } from '@/ui/drain-bar';
import { EDGE_MOTION, type EdgeLeave, useEdgeExit } from '@/ui/edge-card';
import { ChevronDown, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { calendarCopy } from '../copy';
import type { JoinMode } from '../plan';
import { rulesCopy } from '../rules-copy';
import type { CalendarEvent } from '../types';
import { ServiceLogo } from './service-logo';

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
  /** How long the card stays up unanswered, drained along its bottom. */
  openForMs: number;
  onJoin: (mode: JoinMode) => void;
  onDismiss: () => void;
}

/** "In 1 min", or "Starting now" once it has. */
export function reminderLead(event: CalendarEvent, now: Date): string {
  const start = new Date(event.startAt);
  if (start <= now) return copy.started;
  return copy.startsIn(differenceInMinutes(start, now, { roundingMethod: 'ceil' }));
}

/** The first invitees by name (else address), then "+N"; the title's tooltip. */
export function peopleLine(event: CalendarEvent): string {
  const names = event.attendees
    .map((attendee) => attendee.name ?? attendee.email ?? '')
    .filter((name) => name !== '');
  const shown = names.slice(0, NAMED_PEOPLE).join(', ');
  const rest = names.length - NAMED_PEOPLE;
  return rest > 0 ? `${shown} ${copy.others(rest)}` : shown;
}

/**
 * The title over its time, as Granola does; the lead ("In 1 min") and the
 * people are in the tooltip, so the time never truncates.
 */
function Heading({ event, now }: { event: CalendarEvent; now: Date }) {
  const title = event.title === '' ? calendarCopy.untitled : event.title;
  const time = `${format(new Date(event.startAt), 'HH:mm')} – ${format(new Date(event.endAt), 'HH:mm')}`;
  const details = [reminderLead(event, now), peopleLine(event)].filter((part) => part !== '');
  return (
    <div className="min-w-0 flex-1 pr-1" title={[title, ...details].join('\n')}>
      <h1 className="truncate text-sm font-semibold text-fg">{title}</h1>
      <p className="text-xs text-fg-muted tabular-nums">{time}</p>
    </div>
  );
}

/** Props shared by the split button's halves. */
interface PartProps {
  props: ReminderCardProps;
  leave: EdgeLeave;
}

/** The caret's menu: Take notes without joining (with a link), Dismiss. */
function MoreMenu({ props, leave }: PartProps) {
  const hasCall = props.event.conferenceUrl !== null;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        disabled={props.busy}
        aria-label={copy.more}
        className="flex w-8 items-center justify-center border-l border-border text-fg-muted hover:bg-bg-muted disabled:opacity-50"
      >
        <ChevronDown {...ICON_PROPS} className="size-3.5" />
      </DropdownMenuTrigger>
      <DropdownMenuContent side="bottom" align="end">
        {hasCall ? (
          <DropdownMenuItem
            onSelect={() => {
              leave(() => {
                props.onJoin('recordOnly');
              });
            }}
          >
            {copy.withoutJoining}
          </DropdownMenuItem>
        ) : null}
        <DropdownMenuItem
          onSelect={() => {
            leave(props.onDismiss);
          }}
        >
          {copy.dismiss}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Join meeting & open Deino (with a link), or Take notes; and the caret. */
function SplitButton({ props, leave }: PartProps) {
  const { event } = props;
  const hasCall = event.conferenceUrl !== null;
  const mode: JoinMode = hasCall ? 'joinAndRecord' : 'recordOnly';
  return (
    <div className="flex h-10 shrink-0 overflow-hidden rounded-lg border border-border bg-bg shadow-xs">
      <button
        type="button"
        disabled={props.busy}
        onClick={() => {
          leave(() => {
            props.onJoin(mode);
          });
        }}
        className="flex items-center gap-2.5 pr-3 pl-2.5 text-left hover:bg-bg-muted disabled:opacity-50"
      >
        <ServiceLogo platform={hasCall ? event.platform : null} />
        <span className="flex flex-col leading-tight">
          <span className="text-sm font-medium text-fg">
            {hasCall ? copy.join : copy.takeNotes}
          </span>
          {hasCall ? <span className="text-xs text-fg-muted">{copy.andOpenDeino}</span> : null}
        </span>
      </button>
      <MoreMenu props={props} leave={leave} />
    </div>
  );
}

/** The reminder card. */
export function ReminderCard(props: ReminderCardProps) {
  const [leaving, leave] = useEdgeExit(props.failed);
  return (
    <section
      aria-label={copy.label}
      className={cn(
        'relative overflow-hidden rounded-xl bg-surface-raised text-fg shadow-popover',
        leaving ? EDGE_MOTION.exit : EDGE_MOTION.enter,
      )}
    >
      <div className="flex items-center gap-2.5 py-2.5 pr-2.5 pl-3">
        <span aria-hidden className="my-0.5 w-[3px] shrink-0 self-stretch rounded-full bg-brand" />
        <Heading event={props.event} now={props.now} />
        <SplitButton props={props} leave={leave} />
      </div>
      {props.failed ? (
        <p role="alert" className="px-3 pb-2 text-xs text-danger">
          {copy.failed}
        </p>
      ) : null}
      <DrainBar durationMs={props.openForMs} />
    </section>
  );
}
