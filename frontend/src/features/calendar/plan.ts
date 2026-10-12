/**
 * ORG-02's calendar calls and hooks: Coming up with each event's
 * auto-record decision, the record toggle, and Join (which opens the call
 * and starts the recorder in Rust, the same path as the reminder window).
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { commands } from '@/lib/ipc/generated/calendar';
import type {
  CalendarEventId,
  JoinMode,
  MeetingId,
  PlannedEvent,
  RecordChoice,
  RecordDecision,
  TimeRange,
} from '@/lib/ipc/generated/calendar';
import type { Table } from '@/lib/ipc/generated/shared';
import { unwrap } from '@/lib/ipc/result';

import { calendarKeys } from './queries';

/** The tables a plan reads: the events and the series choices. */
const PLAN_TABLES = [
  'calendar_accounts',
  'calendars',
  'calendar_events',
  'calendar_series_choices',
] as const satisfies readonly Table[];

export const planApi = {
  /** The visible events overlapping `range`, each with its decision. */
  plan: (range: TimeRange): Promise<PlannedEvent[]> => unwrap(commands.recordPlan(range)),
  /** The record toggle; resolves to the event's new decision. */
  choose: (id: CalendarEventId, choice: RecordChoice): Promise<RecordDecision> =>
    unwrap(commands.chooseRecord(id, choice)),
  /** Opens the call and/or records it; resolves to the recording meeting. */
  join: (id: CalendarEventId, mode: JoinMode): Promise<MeetingId | null> =>
    unwrap(commands.join(id, mode)),
};

/** The plan for `range`. */
export const recordPlanQuery = (range: TimeRange) =>
  queryOptions({
    queryKey: [...calendarKeys.all, 'plan', range] as const,
    queryFn: () => planApi.plan(range),
    meta: { tables: PLAN_TABLES },
  });

/** The plan for `range`: Coming up with record decisions. */
export function useRecordPlan(range: TimeRange) {
  return useQuery(recordPlanQuery(range));
}

/** The record toggle; the plan refreshes through `DbChanged`, and now. */
export function useChooseRecord() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ id, choice }: { id: CalendarEventId; choice: RecordChoice }) =>
      planApi.choose(id, choice),
    onSettled: () => client.invalidateQueries({ queryKey: [...calendarKeys.all, 'plan'] }),
  });
}

/** Join & record, Join only or Record only. */
export function useJoin() {
  return useMutation({
    mutationFn: ({ id, mode }: { id: CalendarEventId; mode: JoinMode }) => planApi.join(id, mode),
  });
}

export type { JoinMode, PlannedEvent, RecordChoice, RecordDecision };
