import { SkeletonText } from '@/ui/skeleton';

import { permissionsCopy } from '../../copy';
import { missingRequired, upfront } from '../../permission-actions';
import { usePermissions } from '../../queries';
import { SystemPermission } from '../permission-rows';
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
