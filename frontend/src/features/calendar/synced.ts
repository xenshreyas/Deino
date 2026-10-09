/** "Synced 3 minutes ago" labels for the calendar settings cards. */
import { differenceInMinutes, formatDistanceStrict } from 'date-fns';

import { calendarCopy } from './copy';

/** "Synced just now", "Synced 3 minutes ago", or "Not synced yet". */
export function syncedLabel(lastSyncedAt: string | null, now: Date): string {
  if (lastSyncedAt === null) return calendarCopy.sync.neverSynced;
  const syncedAt = new Date(lastSyncedAt);
  if (differenceInMinutes(now, syncedAt) < 1)
    return calendarCopy.sync.synced(calendarCopy.sync.justNow);
  return calendarCopy.sync.synced(formatDistanceStrict(syncedAt, now, { addSuffix: true }));
}
