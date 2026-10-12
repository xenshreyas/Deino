import { SkeletonText } from '@/ui/skeleton';

import { permissionsCopy } from '../../copy';
import { missingRequired } from '../../permission-actions';
import { usePermissions } from '../../queries';
import { CalendarPermission, SystemPermission } from '../permission-rows';
import { SetupList } from '../setup-row';
import { StepFrame, type StepProps } from '../step-frame';

/**
 * Step 2: microphone, system audio, notifications, Accessibility and
 * Screen Recording (both optional) and Calendar, each with its live state and the one button that moves it.
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
        <SetupList label={permissionsCopy.title}>
          {statuses.map((status) => (
            <SystemPermission key={status.kind} status={status} />
          ))}
          <CalendarPermission />
        </SetupList>
      )}
      {statuses !== undefined && missingRequired(statuses) ? (
        <p className="text-sm text-fg-muted">{permissionsCopy.missingRequired}</p>
      ) : null}
    </StepFrame>
  );
}
