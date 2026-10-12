/**
 * Calendar sync (ORG-01): the user's calendars from macOS Calendar, through
 * the `calendar` plugin. Other code imports from here only.
 *
 * - `ComingUp`: the next two weeks, a plain list (the agenda helpers
 *   `agendaDays`, `eventTime`, `startsIn` are shared with Home);
 * - ORG-02: `useRecordPlan` (each event's auto-record decision),
 *   `useChooseRecord` (the record toggle) and `useJoin` (Join & record);
 *   `MeetingReminder` (the reminder window's page), `OpenNoteReporter`
 *   (the meeting page tells Rust which event's note is open),
 *   `useLastMeeting` and `useOpenInCalendar` (Coming up's menu);
 * - `CalendarSettings`: Settings → Calendar (sync switch, macOS access,
 *   the optional Google and Microsoft direct connections, calendars with
 *   colour dots and visibility, refresh, then auto-record and reminders).
 */
export {
  type AgendaDay,
  agendaDays,
  COMING_UP_DAYS,
  comingUpRange,
  eventTime,
  startsIn,
} from './agenda';
export { calendarApi } from './api';
export { useNow } from './hooks/use-now';
export {
  type JoinMode,
  type PlannedEvent,
  planApi,
  type RecordChoice,
  type RecordDecision,
  recordPlanQuery,
  useChooseRecord,
  useJoin,
  useRecordPlan,
} from './plan';
export { CalendarSettings } from './components/calendar-settings';
export { ReminderCalendarList } from './components/rules-settings';
export { ComingUp } from './components/coming-up';
export { MeetingReminder } from './components/meeting-reminder';
export { OpenNoteReporter } from './components/open-note-reporter';
export { type RelatedMeeting, remindersApi, useLastMeeting, useOpenInCalendar } from './reminders';
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
