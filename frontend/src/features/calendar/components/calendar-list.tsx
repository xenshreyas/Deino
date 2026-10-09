/**
 * The calendars of Settings → Calendar, one card per account, each row a
 * colour dot, the calendar's name and a visibility switch. Presentational.
 */
import { Switch } from '@/ui/controls';
import { SettingsCard } from '@/ui/settings-section';

import { calendarCopy } from '../copy';
import type { CalendarId, CalendarView } from '../types';

/** Props of {@link CalendarList}. */
export interface CalendarListProps {
  calendars: readonly CalendarView[];
  onToggle: (id: CalendarId, visible: boolean) => void;
  /** Switches are disabled while a change is being saved. */
  busy?: boolean;
}

/** One account's calendars, in the order Rust lists them. */
interface AccountGroup {
  id: string;
  name: string;
  calendars: CalendarView[];
}

/** The calendars grouped by account, accounts in order of first appearance. */
export function groupByAccount(calendars: readonly CalendarView[]): AccountGroup[] {
  const groups: AccountGroup[] = [];
  for (const calendar of calendars) {
    const group = groups.find((candidate) => candidate.id === calendar.accountId);
    if (group === undefined) {
      groups.push({ id: calendar.accountId, name: calendar.accountName, calendars: [calendar] });
    } else {
      group.calendars.push(calendar);
    }
  }
  return groups;
}

/** One calendar. */
function CalendarRow({
  calendar,
  onToggle,
  busy,
}: {
  calendar: CalendarView;
  onToggle: CalendarListProps['onToggle'];
  busy: boolean;
}) {
  const title = calendar.title === '' ? calendarCopy.calendars.untitled : calendar.title;
  return (
    <div className="flex min-h-[44px] items-center gap-3 px-4 py-2">
      <span
        aria-hidden
        className="size-2.5 shrink-0 rounded-full bg-fg-subtle"
        style={calendar.color === null ? undefined : { backgroundColor: calendar.color }}
      />
      <span className="min-w-0 flex-1 truncate text-sm text-fg">{title}</span>
      <Switch
        aria-label={calendarCopy.calendars.show(title)}
        checked={calendar.visible}
        disabled={busy}
        onCheckedChange={(visible) => {
          onToggle(calendar.id, visible);
        }}
      />
    </div>
  );
}

/** Every account's calendars, one card each. */
export function CalendarList({ calendars, onToggle, busy = false }: CalendarListProps) {
  return (
    <div className="flex flex-col gap-4">
      {groupByAccount(calendars).map((group) => (
        <div key={group.id} className="flex flex-col gap-1.5">
          <h4 className="px-1 text-xs font-semibold text-fg-muted">{group.name}</h4>
          <SettingsCard label={group.name}>
            {group.calendars.map((calendar) => (
              <CalendarRow key={calendar.id} calendar={calendar} onToggle={onToggle} busy={busy} />
            ))}
          </SettingsCard>
        </div>
      ))}
    </div>
  );
}
