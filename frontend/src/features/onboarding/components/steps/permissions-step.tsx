import { SkeletonText } from '@/ui/skeleton';

import { permissionsCopy } from '../../copy';
import { missingRequired, upfront } from '../../permission-actions';
import { PERMISSION_ICONS } from '../../permission-icons';
import { useOpenPermissionSettings, usePermissions, useRequestPermission } from '../../queries';
import type { PermissionStatus } from '../../types';
import { PermissionRow } from '../permission-row';
import { SetupList } from '../setup-row';
import { StepFrame, type StepProps } from '../step-frame';

/**
 * Step 2: the microphone and system audio, each with its live state and the
 * one button that moves it (UX-22). Calendar has its own step (UX-21), and
 * the optional permissions are asked where they matter. The list re-reads
 * itself, so a switch flipped in System Settings shows up.
 */
export function PermissionsStep({ dispatch }: StepProps) {
  const permissions = usePermissions();
  const statuses = permissions.data === undefined ? undefined : upfront(permissions.data);
  return (
    <StepFrame
      step="permissions"
      title={permissionsCopy.title}
      description={permissionsCopy.description}
      dispatch={dispatch}
    >
      {statuses === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <SetupList label={permissionsCopy.title}>
          {statuses.map((status) => (
            <SystemPermission key={status.kind} status={status} />
          ))}
        </SetupList>
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
      icon={PERMISSION_ICONS[status.kind]}
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
