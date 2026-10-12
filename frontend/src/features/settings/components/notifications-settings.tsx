/**
 * Settings → General → Notifications (UX-17): one switch per kind of
 * notification, in one place. The settings keys stay where they were
 * (`notifications.preferences`, `calendar.reminders.enabled`); which
 * calendars remind is under "More options".
 */
import { ReminderCalendarList } from '@/features/calendar';
import { MoreOptions } from '@/ui/more-options';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { notificationsCopy as copy } from '../copy';
import { useSettings } from '../queries';
import type { Settings } from '../types';
import { useSaveSettings } from './save-settings';
import { SwitchRow } from './switch-row';

/** The three switches over the saved settings. */
function NotificationRows({ settings }: { settings: Settings }) {
  const { save } = useSaveSettings();
  const { preferences } = settings.notifications;
  return (
    <SettingsCard label={copy.title}>
      <SwitchRow
        id="notify-recording"
        label={copy.recordingTitle}
        description={copy.recordingDescription}
        checked={preferences.showRecordingStarted && preferences.showRecordingStopped}
        onChange={(on) => {
          const both = { showRecordingStarted: on, showRecordingStopped: on };
          save({ notifications: { preferences: both } });
        }}
      />
      <SwitchRow
        id="notify-meetings"
        label={copy.remindersTitle}
        description={copy.remindersDescription}
        checked={settings.calendar.reminders.enabled}
        onChange={(enabled) => {
          save({ calendar: { reminders: { enabled } } });
        }}
      />
      <SwitchRow
        id="notify-action-items"
        label={copy.actionItemsTitle}
        description={copy.actionItemsDescription}
        checked={preferences.showActionItemReminders}
        onChange={(showActionItemReminders) => {
          save({ notifications: { preferences: { showActionItemReminders } } });
        }}
      />
    </SettingsCard>
  );
}

/** The Notifications section. */
export function NotificationsSettings() {
  const { data: settings } = useSettings();
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {settings === undefined ? (
        <SkeletonText lines={3} />
      ) : (
        <>
          <NotificationRows settings={settings} />
          <MoreOptions label={copy.moreLabel}>
            <ReminderCalendarList />
          </MoreOptions>
        </>
      )}
    </SettingsSection>
  );
}
