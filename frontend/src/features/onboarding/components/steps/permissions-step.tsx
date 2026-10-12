import { useOpenAccessSettings, useCalendarStatus, useSetCalendarSync } from '@/features/calendar';
import { AudioLines, Bell, Calendar, type Icon, Mic, Users } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { permissionsCopy } from '../../copy';
import { calendarPermission, missingRequired } from '../../permission-actions';
import { useOpenPermissionSettings, usePermissions, useRequestPermission } from '../../queries';
import type { PermissionKind, PermissionStatus } from '../../types';
import { PermissionRow } from '../permission-row';
import { StepFrame, type StepProps } from '../step-frame';

/** Each permission's icon. */
const ICONS: Record<PermissionKind, Icon> = {
  microphone: Mic,
  system_audio: AudioLines,
  notifications: Bell,
  accessibility: Users,
};

/**
 * Step 2: microphone, system audio, notifications, Accessibility (optional)
 * and Calendar, each with its live state and the one button that moves it.
 * The list re-reads itself, so a switch flipped in System Settings shows up.
 */
export function PermissionsStep({ dispatch }: StepProps) {
  const permissions = usePermissions();
  const statuses = permissions.data;
  return (
    <StepFrame
      step="permissions"
      title={permissionsCopy.title}
      description={permissionsCopy.description}
      dispatch={dispatch}
    >
      {statuses === undefined ? (
        <SkeletonText lines={5} />
      ) : (
        <ul className="flex flex-col divide-y divide-border border-y border-border">
          {statuses.map((status) => (
            <SystemPermission key={status.kind} status={status} />
          ))}
          <CalendarPermission />
        </ul>
      )}
      {statuses !== undefined && missingRequired(statuses) ? (
        <p className="text-sm text-fg-muted">{permissionsCopy.missingRequired}</p>
      ) : null}
    </StepFrame>
  );
}

/** A permission the `onboarding` plugin reads and requests. */
function SystemPermission({ status }: { status: PermissionStatus }) {
  const request = useRequestPermission();
  const settings = useOpenPermissionSettings();
  const copy = permissionsCopy.kinds[status.kind];
  return (
    <PermissionRow
      icon={ICONS[status.kind]}
      name={copy.name}
      why={copy.why}
      state={status.state}
      optional={status.optional}
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
function CalendarPermission() {
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
      optional
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
