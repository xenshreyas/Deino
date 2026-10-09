/**
 * TanStack Query hooks for calendar sync.
 *
 * Calendars and events are tables, so `DbChanged` refreshes their queries
 * after every sync that changed something. The status (the switch, macOS
 * access, the last sync) and which calendars are hidden are not rows: the
 * typed `CalendarChanged` event carries the new status and refreshes the
 * lists ({@link useCalendarChanges}), and every mutation writes its result
 * straight into the cache.
 */
import {
  queryOptions,
  type QueryClient,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import { useIpcEvent } from '@/lib/ipc/events';
import { events } from '@/lib/ipc/generated/calendar';
import type { Table } from '@/lib/ipc/generated/shared';

import { calendarApi } from './api';
import type {
  CalendarId,
  CalendarProvider,
  CalendarStatus,
  CalendarView,
  TimeRange,
} from './types';

/** The tables the calendar lists read. */
const CALENDAR_TABLES = [
  'calendar_accounts',
  'calendars',
  'calendar_events',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `calendarKeys.all`. */
export const calendarKeys = {
  all: ['calendar'] as const,
  status: () => [...calendarKeys.all, 'status'] as const,
  calendars: () => [...calendarKeys.all, 'calendars'] as const,
  upcomingAll: () => [...calendarKeys.all, 'upcoming'] as const,
  upcoming: (range: TimeRange) => [...calendarKeys.upcomingAll(), range.start, range.end] as const,
};

/** Whether sync is on, macOS access and the last sync. */
export const calendarStatusQuery = () =>
  queryOptions({ queryKey: calendarKeys.status(), queryFn: calendarApi.status });

/** Every calendar with its account, colour and visibility. */
export const calendarsQuery = () =>
  queryOptions({
    queryKey: calendarKeys.calendars(),
    queryFn: calendarApi.calendars,
    meta: { tables: CALENDAR_TABLES },
  });

/** The visible events overlapping `range`. */
export const upcomingQuery = (range: TimeRange) =>
  queryOptions({
    queryKey: calendarKeys.upcoming(range),
    queryFn: () => calendarApi.upcoming(range),
    meta: { tables: CALENDAR_TABLES },
  });

/** Whether sync is on, macOS access and the last sync. */
export function useCalendarStatus() {
  return useQuery(calendarStatusQuery());
}

/** Every calendar with its account, colour and visibility. */
export function useCalendars() {
  return useQuery(calendarsQuery());
}

/** The visible events overlapping `range`. */
export function useUpcoming(range: TimeRange) {
  return useQuery(upcomingQuery(range));
}

/** Caches `status` and refetches the lists that depend on it. */
function applyStatus(client: QueryClient, status: CalendarStatus): void {
  client.setQueryData(calendarKeys.status(), status);
  void client.invalidateQueries({ queryKey: calendarKeys.calendars() });
  void client.invalidateQueries({ queryKey: calendarKeys.upcomingAll() });
}

/** Keeps the status and lists current while mounted: every `CalendarChanged`. */
export function useCalendarChanges(): void {
  const client = useQueryClient();
  useIpcEvent(events.calendarChanged, ({ status }) => {
    applyStatus(client, status);
  });
}

/** Turns sync on (macOS may ask for access) or off. */
export function useSetCalendarSync() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (enabled: boolean) => (enabled ? calendarApi.enable() : calendarApi.disable()),
    onSuccess: (status) => {
      applyStatus(client, status);
    },
  });
}

/** Syncs now. */
export function useRefreshCalendars() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: calendarApi.refresh,
    onSuccess: (status) => {
      applyStatus(client, status);
    },
  });
}

/** Input of {@link useConnectCalendar}. */
export interface ConnectorChange {
  provider: CalendarProvider;
  /** Connect (`true`, opens the browser) or disconnect (`false`). */
  connected: boolean;
}

/** Connects or disconnects a Google or Microsoft account. */
export function useConnectCalendar() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ provider, connected }: ConnectorChange) =>
      connected ? calendarApi.connect(provider) : calendarApi.disconnect(provider),
    onSuccess: (status) => {
      applyStatus(client, status);
    },
  });
}

/** Opens System Settings at Calendars access. */
export function useOpenAccessSettings() {
  return useMutation({ mutationFn: calendarApi.openAccessSettings });
}

/** Input of {@link useSetCalendarVisible}. */
export interface CalendarVisibility {
  id: CalendarId;
  visible: boolean;
}

/** Shows or hides a calendar and caches the result. */
export function useSetCalendarVisible() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ id, visible }: CalendarVisibility) =>
      calendarApi.setCalendarVisible(id, visible),
    onSuccess: (updated) => {
      client.setQueryData(calendarKeys.calendars(), (calendars: CalendarView[] | undefined) =>
        calendars?.map((calendar) => (calendar.id === updated.id ? updated : calendar)),
      );
      void client.invalidateQueries({ queryKey: calendarKeys.upcomingAll() });
    },
  });
}
