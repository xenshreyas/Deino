import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { CircleCheck, type Icon, ICON_PROPS } from '@/ui/icons';

import { permissionsCopy } from '../copy';
import { actionFor } from '../permission-actions';
import type { PermissionState } from '../types';
import { SetupRow } from './setup-row';

/** Props of {@link PermissionRow}. */
export interface PermissionRowProps {
  icon: Icon;
  name: string;
  why: string;
  state: PermissionState;
  optional?: boolean;
  /** A request or settings call for this row is running. */
  pending?: boolean;
  onRequest: () => void;
  onOpenSettings: () => void;
}

/**
 * One permission: what it is for, where it stands, and the one button that
 * moves it forward (ask, or open System Settings once refused).
 */
export function PermissionRow({ icon, name, why, state, ...props }: PermissionRowProps) {
  const label = (
    <>
      {name}
      {props.optional === true ? (
        <span className="text-xs font-normal text-fg-subtle">{permissionsCopy.optional}</span>
      ) : null}
    </>
  );
  return (
    <SetupRow icon={icon} name={label} detail={why}>
      <RowControl state={state} {...props} />
    </SetupRow>
  );
}

type RowControlProps = Pick<
  PermissionRowProps,
  'state' | 'pending' | 'onRequest' | 'onOpenSettings'
>;

/** "Allowed", or the state and the button that changes it. */
function RowControl({ state, pending = false, onRequest, onOpenSettings }: RowControlProps) {
  const action = actionFor(state);
  if (action === 'none') {
    return (
      <span className="flex shrink-0 items-center gap-1.5 text-sm text-fg-muted">
        {state === 'granted' ? <CircleCheck {...ICON_PROPS} className="text-accent" /> : null}
        {permissionsCopy.states[state]}
      </span>
    );
  }
  const settings = action === 'settings';
  return (
    <span className="flex shrink-0 items-center gap-3">
      <span className={cn('text-xs', settings ? 'text-danger' : 'text-fg-subtle')}>
        {permissionsCopy.states[state]}
      </span>
      <Button
        variant="secondary"
        size="sm"
        disabled={pending}
        onClick={settings ? onOpenSettings : onRequest}
      >
        {settings ? permissionsCopy.openSettings : permissionsCopy.allow}
      </Button>
    </span>
  );
}
