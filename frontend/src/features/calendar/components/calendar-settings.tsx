/**
 * Settings → Calendar (ORG-01): turn macOS Calendar sync on or off, see what
 * macOS allows and when the last sync ran, connect Google or Microsoft
 * directly, choose which calendars show, then (ORG-02) which meetings
 * record on their own. Meeting reminders are Settings → General →
 * Notifications (UX-17).
 * Container: state comes from the `calendar` plugin's queries, kept current
 * by `CalendarChanged` and `DbChanged`.
 */
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { calendarCopy } from '../copy';
import { useNow } from '../hooks/use-now';
import {
  useCalendarChanges,
  useCalendars,
  useCalendarStatus,
  useOpenAccessSettings,
  useRefreshCalendars,
  useSetCalendarSync,
  useSetCalendarVisible,
} from '../queries';
import { CalendarList } from './calendar-list';
import { ConnectorsSection } from './connectors-section';
import { AutoRecordSection } from './rules-settings';
import { SyncCard } from './sync-card';

/** The macOS Calendar section: the switch, access and the last sync. */
function SyncSection() {
  const now = useNow();
  const status = useCalendarStatus();
  const setSync = useSetCalendarSync();
  const refresh = useRefreshCalendars();
  const openSettings = useOpenAccessSettings();
  const failure = setSync.isError ? calendarCopy.sync.switchFailed : null;
  const settingsFailure = openSettings.data === false || openSettings.isError;
  return (
    <SettingsSection title={calendarCopy.sync.title} description={calendarCopy.sync.description}>
      {status.data === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <SyncCard
          status={status.data}
          now={now}
          busy={setSync.isPending || refresh.isPending}
          onToggle={setSync.mutate}
          onRefresh={() => {
            refresh.mutate();
          }}
          onOpenSettings={() => {
            openSettings.mutate();
          }}
        />
      )}
      {failure === null ? null : (
        <p role="alert" className="text-xs text-danger">
          {failure}
        </p>
      )}
      {settingsFailure ? (
        <p role="alert" className="text-xs text-danger">
          {calendarCopy.sync.settingsFailed}
        </p>
      ) : null}
    </SettingsSection>
  );
}

/** The calendars section: one card per account with visibility switches. */
function CalendarsSection() {
  const calendars = useCalendars();
  const toggle = useSetCalendarVisible();
  return (
    <SettingsSection
      title={calendarCopy.calendars.title}
      description={calendarCopy.calendars.description}
    >
      {calendars.isPending ? <SkeletonText lines={3} /> : null}
      {calendars.isError ? (
        <p role="alert" className="text-sm text-danger">
          {calendarCopy.calendars.loadFailed}
        </p>
      ) : null}
      {calendars.data?.length === 0 ? (
        <p className="text-sm text-fg-muted">{calendarCopy.calendars.empty}</p>
      ) : null}
      {calendars.data === undefined || calendars.data.length === 0 ? null : (
        <CalendarList
          calendars={calendars.data}
          busy={toggle.isPending}
          onToggle={(id, visible) => {
            toggle.mutate({ id, visible });
          }}
        />
      )}
      {toggle.isError ? (
        <p role="alert" className="text-xs text-danger">
          {calendarCopy.calendars.toggleFailed}
        </p>
      ) : null}
    </SettingsSection>
  );
}

/**
 * The Calendar settings page. Onboarding hides the direct Google and
 * Microsoft connections and auto-record (`connectors={false}`):
 * they are no first-run task.
 */
export function CalendarSettings({ connectors = true }: { connectors?: boolean }) {
  useCalendarChanges();
  return (
    <div className="flex flex-col gap-8">
      <SyncSection />
      {connectors ? <ConnectorsSection /> : null}
      <CalendarsSection />
      {connectors ? <AutoRecordSection /> : null}
    </div>
  );
}
