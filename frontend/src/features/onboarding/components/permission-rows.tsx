/**
 * The permission rows onboarding's Permissions step and Settings →
 * Recording → Permissions (UX-10) share: each macOS permission once, with
 * its live state and the one button that moves it.
 */
import { useCalendarStatus, useOpenAccessSettings, useSetCalendarSync } from '@/features/calendar';
import { Calendar } from '@/ui/icons';

import { permissionsCopy } from '../copy';
import { calendarPermission } from '../permission-actions';
import { PERMISSION_ICONS } from '../permission-icons';
import { useOpenPermissionSettings, useRequestPermission } from '../queries';
import type { PermissionStatus } from '../types';
import { PermissionRow } from './permission-row';

/** Props of {@link SystemPermission}. */
export interface SystemPermissionProps {
  status: PermissionStatus;
  /** Mark optional permissions as such (onboarding does; Settings does not). */
  showOptional?: boolean;
}

/** A permission the `onboarding` plugin reads and requests. */
export function SystemPermission({ status, showOptional = true }: SystemPermissionProps) {
  const request = useRequestPermission();
  const settings = useOpenPermissionSettings();
  const copy = permissionsCopy.kinds[status.kind];
  return (
    <PermissionRow
      icon={PERMISSION_ICONS[status.kind]}
      name={copy.name}
      why={copy.why}
      state={status.state}
      optional={showOptional && status.optional}
      pending={request.isPending || settings.isPending}
      onRequest={() => {
        request.mutate(status.kind);
      }}
      onOpenSettings={() => {
        settings.mutate(status.kind);
      }}
    />
  );
}

/** macOS Calendar access, through the calendar feature that owns it. */
export function CalendarPermission({ showOptional = true }: { showOptional?: boolean }) {
  const status = useCalendarStatus();
  const enable = useSetCalendarSync();
  const settings = useOpenAccessSettings();
  if (status.data === undefined) return null;
  return (
    <PermissionRow
      icon={Calendar}
      name={permissionsCopy.calendar.name}
      why={permissionsCopy.calendar.why}
      state={calendarPermission(status.data.access)}
      optional={showOptional}
      pending={enable.isPending || settings.isPending}
      onRequest={() => {
        enable.mutate(true);
      }}
      onOpenSettings={() => {
        settings.mutate();
      }}
    />
  );
}
