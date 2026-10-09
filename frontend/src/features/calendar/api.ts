/**
 * Typed calls to the `calendar` plugin: the only place this feature touches
 * IPC. Each resolves to the data or throws an `IpcError`.
 */
import { commands } from '@/lib/ipc/generated/calendar';
import { unwrap } from '@/lib/ipc/result';

import type {
  CalendarEvent,
  CalendarEventId,
  CalendarId,
  CalendarStatus,
  CalendarView,
  TimeRange,
} from './types';

export const calendarApi = {
  /** Whether sync is on, macOS Calendars access, and the last sync. */
  status: (): Promise<CalendarStatus> => unwrap(commands.status()),
  /** Turns sync on; the first time, macOS asks the user for access. */
  enable: (): Promise<CalendarStatus> => unwrap(commands.enable()),
  /** Turns sync off; synced events stay for linked meetings. */
  disable: (): Promise<CalendarStatus> => unwrap(commands.disable()),
  /** Syncs now and resolves to the new status. */
  refresh: (): Promise<CalendarStatus> => unwrap(commands.refresh()),
  /** Opens System Settings at Calendars access; `false` when it could not. */
  openAccessSettings: (): Promise<boolean> => unwrap(commands.openAccessSettings()),
  /** Every calendar with its account, colour and visibility. */
  calendars: (): Promise<CalendarView[]> => unwrap(commands.listCalendars()),
  /** Shows or hides a calendar's events. */
  setCalendarVisible: (id: CalendarId, visible: boolean): Promise<CalendarView> =>
    unwrap(commands.setCalendarVisible(id, visible)),
  /** The visible events overlapping `range`, by start. */
  upcoming: (range: TimeRange): Promise<CalendarEvent[]> => unwrap(commands.upcoming(range)),
  /** One synced event. */
  event: (id: CalendarEventId): Promise<CalendarEvent> => unwrap(commands.event(id)),
};
