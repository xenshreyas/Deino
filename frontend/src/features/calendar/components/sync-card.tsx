/**
 * The macOS Calendar card of Settings → Calendar: the sync switch with what
 * macOS allows, a way to System Settings when access is off, and when the
 * last sync ran. Presentational.
 */
import { differenceInMinutes, formatDistanceStrict } from 'date-fns';

import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { RotateCcw, ICON_PROPS } from '@/ui/icons';
import { SettingsCard } from '@/ui/settings-section';

import { calendarCopy } from '../copy';
import type { CalendarAccess, CalendarStatus } from '../types';

/** Access states that only System Settings can change. */
const FIXED_IN_SETTINGS: readonly CalendarAccess[] = ['denied', 'write_only'];

/** Props of {@link SyncCard}. */
export interface SyncCardProps {
  status: CalendarStatus;
  now: Date;
  /** A switch or refresh is in flight. */
  busy: boolean;
  onToggle: (enabled: boolean) => void;
  onRefresh: () => void;
  onOpenSettings: () => void;
}

/** "Synced 3 minutes ago", or "Not synced yet". */
function lastSynced(status: CalendarStatus, now: Date): string {
  if (status.lastSyncedAt === null) return calendarCopy.sync.neverSynced;
  const syncedAt = new Date(status.lastSyncedAt);
  if (differenceInMinutes(now, syncedAt) < 1)
    return calendarCopy.sync.synced(calendarCopy.sync.justNow);
  const when = formatDistanceStrict(syncedAt, now, { addSuffix: true });
  return calendarCopy.sync.synced(when);
}

/** The switch row. */
function SwitchRow({
  status,
  busy,
  onToggle,
}: Pick<SyncCardProps, 'status' | 'busy' | 'onToggle'>) {
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div>
        <p id="calendar-sync-label" className="text-sm text-fg">
          {calendarCopy.sync.label}
        </p>
        <p className="text-xs text-fg-muted">{calendarCopy.sync.access[status.access]}</p>
      </div>
      <Switch
        aria-labelledby="calendar-sync-label"
        checked={status.syncEnabled}
        disabled={busy || status.access === 'unsupported'}
        onCheckedChange={onToggle}
      />
    </div>
  );
}

/** The last sync, its error, and Refresh. */
function StatusRow({
  status,
  now,
  busy,
  onRefresh,
}: Pick<SyncCardProps, 'status' | 'now' | 'busy' | 'onRefresh'>) {
  return (
    <div className="flex min-h-[44px] items-center justify-between gap-4 px-4 py-2">
      <div className="min-w-0">
        <p className="text-xs text-fg-muted">
          {busy ? calendarCopy.sync.refreshing : lastSynced(status, now)}
        </p>
        {status.lastError === null ? null : (
          <p role="alert" className="text-xs text-danger">
            {status.lastError}
          </p>
        )}
      </div>
      <Button variant="ghost" size="sm" disabled={busy} onClick={onRefresh}>
        <RotateCcw {...ICON_PROPS} />
        {calendarCopy.sync.refresh}
      </Button>
    </div>
  );
}

/** The macOS Calendar card. */
export function SyncCard(props: SyncCardProps) {
  const { status, onOpenSettings } = props;
  return (
    <SettingsCard label={calendarCopy.sync.title}>
      <SwitchRow {...props} />
      {FIXED_IN_SETTINGS.includes(status.access) ? (
        <div className="px-4 py-3">
          <Button variant="secondary" size="sm" onClick={onOpenSettings}>
            {calendarCopy.sync.openSettings}
          </Button>
        </div>
      ) : null}
      {status.syncEnabled ? <StatusRow {...props} /> : null}
    </SettingsCard>
  );
}
