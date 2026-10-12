/**
 * ORG-02's rules over the typed settings: Settings → Calendar →
 * "Auto-record" (the four switches of `calendar.autoRecord`), and the
 * per-calendar reminder choices (`calendar.reminders.quietCalendars`) that
 * Settings → General → Notifications shows under "More options" (UX-17),
 * beside its reminders switch (`calendar.reminders.enabled`).
 */
import { toast } from 'sonner';

import {
  type Settings,
  type SettingsPatch,
  usePatchSettings,
  useSettings,
} from '@/features/settings';
import { Switch } from '@/ui/controls';
import { MoreOptions } from '@/ui/more-options';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { calendarCopy } from '../copy';
import { useCalendars } from '../queries';
import { rulesCopy } from '../rules-copy';
import type { CalendarId, CalendarView } from '../types';

type CalendarSettings = Settings['calendar'];
type AutoRecord = CalendarSettings['autoRecord'];
type Reminders = CalendarSettings['reminders'];

/** The four switches, in the order Settings shows them. */
const SWITCHES = [
  ['internal', 'internalHint'],
  ['external', 'externalHint'],
  ['organizedByOthers', 'organizedByOthersHint'],
  ['notAccepted', 'notAcceptedHint'],
] as const satisfies readonly (readonly [keyof AutoRecord, keyof typeof rulesCopy.autoRecord])[];

/** A patch turning switch `key` `on` or off. */
function flag(key: keyof AutoRecord, on: boolean): Partial<AutoRecord> {
  const patch: Partial<AutoRecord> = {};
  patch[key] = on;
  return patch;
}

/** Saves a calendar settings patch, toasting `failed` when it does not take. */
function useSaveCalendar(failed: string) {
  const patch = usePatchSettings();
  const save = (calendar: NonNullable<SettingsPatch['calendar']>) => {
    patch.mutate(
      { calendar },
      { onError: (error) => toast.error(failed, { description: error.message }) },
    );
  };
  return { save, busy: patch.isPending };
}

function AutoRecordCard({ rules }: { rules: AutoRecord }) {
  const copy = rulesCopy.autoRecord;
  const { save, busy } = useSaveCalendar(copy.saveFailed);
  return (
    <SettingsCard label={copy.title}>
      {SWITCHES.map(([key, hint]) => (
        <SettingsRow
          key={key}
          label={copy[key]}
          description={copy[hint]}
          htmlFor={`auto-record-${key}`}
        >
          <Switch
            id={`auto-record-${key}`}
            checked={rules[key]}
            disabled={busy}
            onCheckedChange={(on) => {
              save({ autoRecord: flag(key, on) });
            }}
          />
        </SettingsRow>
      ))}
    </SettingsCard>
  );
}

/** "Auto-record", its four rules under a closed disclosure (UX-15). */
export function AutoRecordSection() {
  const settings = useSettings();
  const rules = settings.data?.calendar.autoRecord;
  return (
    <SettingsSection
      title={rulesCopy.autoRecord.title}
      description={rulesCopy.autoRecord.description}
    >
      {rules === undefined ? (
        <SkeletonText lines={1} />
      ) : (
        <MoreOptions label={rulesCopy.autoRecord.choose}>
          <AutoRecordCard rules={rules} />
        </MoreOptions>
      )}
    </SettingsSection>
  );
}

/** `quiet` with `id` added (`remind` off) or removed (on). */
export function withReminder(
  quiet: readonly CalendarId[],
  id: CalendarId,
  remind: boolean,
): CalendarId[] {
  const others = quiet.filter((candidate) => candidate !== id);
  return remind ? others : [...others, id];
}

function CalendarReminders(props: { calendars: readonly CalendarView[]; reminders: Reminders }) {
  const copy = rulesCopy.reminders;
  const { save, busy } = useSaveCalendar(copy.saveFailed);
  const quiet = props.reminders.quietCalendars;
  return (
    <SettingsCard label={copy.calendars}>
      {props.calendars.map((calendar) => {
        const title = calendar.title === '' ? calendarCopy.calendars.untitled : calendar.title;
        return (
          <SettingsRow
            key={calendar.id}
            label={title}
            description={calendar.accountName}
            htmlFor={`remind-${calendar.id}`}
          >
            <Switch
              id={`remind-${calendar.id}`}
              aria-label={copy.remindFor(title)}
              checked={!quiet.includes(calendar.id)}
              disabled={busy || !props.reminders.enabled}
              onCheckedChange={(remind) => {
                save({ reminders: { quietCalendars: withReminder(quiet, calendar.id, remind) } });
              }}
            />
          </SettingsRow>
        );
      })}
    </SettingsCard>
  );
}

/** Per calendar, whether it reminds, or a line while no calendar is shown. */
export function ReminderCalendarList() {
  const settings = useSettings();
  const calendars = useCalendars();
  const visible = (calendars.data ?? []).filter((calendar) => calendar.visible);
  const reminders = settings.data?.calendar.reminders;
  if (reminders === undefined) return <SkeletonText lines={2} />;
  return visible.length === 0 ? (
    <p className="text-sm text-fg-muted">{calendarCopy.calendars.empty}</p>
  ) : (
    <CalendarReminders calendars={visible} reminders={reminders} />
  );
}
