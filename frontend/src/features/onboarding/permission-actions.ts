/**
 * What the permissions step offers for each permission: pure rules over the
 * states Rust reports, so the components only render them.
 */
import type { CalendarAccess } from '@/features/calendar';

import type { PermissionKind, PermissionState, PermissionStatus } from './types';

/** The one button a permission row shows. */
export type RowAction =
  /** Nothing to do: granted, or not needed on this platform. */
  | 'none'
  /** Ask: macOS shows its prompt (or, for system audio, starts the probe). */
  | 'request'
  /** Refused: only System Settings can change it now. */
  | 'settings';

/** The button for a permission in `state`. */
export function actionFor(state: PermissionState): RowAction {
  switch (state) {
    case 'granted':
    case 'unsupported':
      return 'none';
    case 'not_determined':
    case 'unknown':
      return 'request';
    case 'denied':
      return 'settings';
  }
}

/**
 * Calendar access in the permission vocabulary. Write-only access cannot
 * read events, so it counts as refused.
 */
export function calendarPermission(access: CalendarAccess): PermissionState {
  switch (access) {
    case 'granted':
      return 'granted';
    case 'not_determined':
      return 'not_determined';
    case 'denied':
    case 'restricted':
    case 'write_only':
      return 'denied';
    case 'unsupported':
      return 'unsupported';
  }
}

/** Whether a permission recording needs is still refused or unanswered. */
export function missingRequired(statuses: readonly PermissionStatus[]): boolean {
  return statuses.some((status) => !status.optional && actionFor(status.state) !== 'none');
}

/**
 * The permissions setup asks for (UX-22): only what recording needs. The
 * rest are asked where they matter: Accessibility on the first Zoom, Teams
 * or Meet call, Notifications beside the first upcoming meeting, Screen
 * Recording when screen capture is turned on.
 */
export const UPFRONT_PERMISSIONS: readonly PermissionKind[] = ['microphone', 'system_audio'];

/** The statuses setup shows, in {@link UPFRONT_PERMISSIONS} order. */
export function upfront(statuses: readonly PermissionStatus[]): PermissionStatus[] {
  return UPFRONT_PERMISSIONS.flatMap((kind) => statuses.filter((status) => status.kind === kind));
}
