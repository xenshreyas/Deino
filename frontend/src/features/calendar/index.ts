/**
 * Calendar sync (ORG-01): the user's calendars from macOS Calendar, through
 * the `calendar` plugin. Other code imports from here only.
 *
 * - `ComingUp`: the next two weeks, a simple list ORG-02 places on Home
 *   (shown on the current Home while nothing records);
 * - `CalendarSettings`: Settings → Calendar (sync switch, macOS access,
 *   the optional Google and Microsoft direct connections, calendars with
 *   colour dots and visibility, refresh).
 */
export { calendarApi } from './api';
export { CalendarSettings } from './components/calendar-settings';
export { ComingUp } from './components/coming-up';
export {
  calendarKeys,
  calendarsQuery,
  calendarStatusQuery,
  upcomingQuery,
  useCalendarChanges,
  useCalendars,
  useCalendarStatus,
  useOpenAccessSettings,
  useSetCalendarSync,
  useUpcoming,
} from './queries';
export type {
  CalendarAccess,
  CalendarEvent,
  CalendarEventId,
  CalendarId,
  CalendarStatus,
  CalendarView,
  TimeRange,
} from './types';
