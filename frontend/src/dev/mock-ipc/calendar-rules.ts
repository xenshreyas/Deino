/**
 * ORG-02's calendar commands for `pnpm dev:mock`: each event's auto-record
 * decision (the Rust policy's precedence: this event, then its series,
 * declined, not a meeting, then the switches in the mock settings), the
 * record toggle, and Join, which starts the mock recorder.
 */
import type * as Generated from '@/lib/ipc/generated/calendar';

import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import { mockCalendarEvents as calendarEvents } from './database';
import { latency } from './latency';
import { commands as recording } from './recording';
import { currentSettings } from './settings-store';

type CalendarEvent = Generated.CalendarEvent;
type RecordDecision = Generated.RecordDecision;

/** "All occurrences" choices by series. */
const seriesChoices = new Map<string, boolean>();

const decision = (record: boolean, reason: Generated.RecordReason): RecordDecision => ({
  record,
  reason,
});

/** The mock's user organises events without an organiser or with an `@deino.dev` one. */
function organizedByMe(event: CalendarEvent): boolean {
  return event.organizerEmail === null || event.organizerEmail.endsWith('@deino.dev');
}

/** Step 5 of `deino_core::calendar::rules::decide`: the switches. */
function switches(event: CalendarEvent): RecordDecision {
  const rules = currentSettings().calendar.autoRecord;
  if (!(event.isExternal ? rules.external : rules.internal)) return decision(false, 'audienceOff');
  const mine = organizedByMe(event);
  if (!mine && !rules.organizedByOthers) return decision(false, 'organizedByOthers');
  const accepted = mine || event.myRsvp === 'accepted';
  if (!accepted && !rules.notAccepted) return decision(false, 'notAccepted');
  return decision(true, 'rules');
}

/** `deino_core::calendar::rules::decide`, over the mock settings. */
export function mockDecide(event: CalendarEvent): RecordDecision {
  if (event.recordOverride !== null) return decision(event.recordOverride, 'eventChoice');
  const series = event.seriesId === null ? undefined : seriesChoices.get(event.seriesId);
  if (series !== undefined) return decision(series, 'seriesChoice');
  if (event.myRsvp === 'declined') return decision(false, 'declined');
  if (event.isAllDay || event.attendees.length < 2) return decision(false, 'notAMeeting');
  return switches(event);
}

/** Applies a record toggle to the stored events, like `AutoRecordService::choose`. */
function applyChoice(event: CalendarEvent, choice: Generated.RecordChoice): void {
  if (choice.scope === 'thisEvent' || event.seriesId === null) {
    event.recordOverride = choice.record;
    return;
  }
  const series = event.seriesId;
  if (choice.record === null) {
    seriesChoices.delete(series);
    event.recordOverride = null;
    return;
  }
  seriesChoices.set(series, choice.record);
  for (const other of calendarEvents) {
    if (other.seriesId === series) other.recordOverride = null;
  }
}

function notFound<T>(id: string): CommandResult<T> {
  return failure<T>({ code: 'not_found', message: `calendar event ${id} was not found` });
}

type RulesCommands = Pick<typeof Generated.commands, 'recordPlan' | 'chooseRecord' | 'join'>;

/** The ORG-02 commands; `upcoming` lists the events `recordPlan` decides. */
export function rulesCommands(upcoming: typeof Generated.commands.upcoming): RulesCommands {
  return {
    async recordPlan(range: Generated.TimeRange) {
      const listed = await upcoming(range);
      if (listed.status === 'error') return listed;
      return ok(listed.data.map((event) => ({ event, decision: mockDecide(event) })));
    },
    async chooseRecord(id: Generated.CalendarEventId, choice: Generated.RecordChoice) {
      await latency();
      const event = calendarEvents.find((candidate) => candidate.id === id);
      if (event === undefined) return notFound(id);
      applyChoice(event, choice);
      return ok(mockDecide(event));
    },
    async join(id: Generated.CalendarEventId, mode: Generated.JoinMode) {
      await latency();
      const event = calendarEvents.find((candidate) => candidate.id === id);
      if (event === undefined) return notFound(id);
      if (mode === 'joinOnly' && event.conferenceUrl === null) {
        return failure<Generated.MeetingId | null>({
          code: 'invalid',
          message: 'this event has no call link',
        });
      }
      if (mode === 'joinOnly') return ok(null);
      return recording.start(event.title === '' ? null : event.title);
    },
  };
}
