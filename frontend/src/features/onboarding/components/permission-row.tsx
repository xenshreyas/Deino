import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { CircleCheck, type Icon, ICON_PROPS } from '@/ui/icons';

import { permissionsCopy } from '../copy';
import { actionFor } from '../permission-actions';
import type { PermissionState } from '../types';

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
export function PermissionRow({ icon: RowIcon, name, why, state, ...props }: PermissionRowProps) {
  return (
    <li className="flex items-center gap-4 py-4">
      <RowIcon {...ICON_PROPS} className="shrink-0 text-fg-muted" />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="flex items-center gap-2 text-sm font-semibold text-fg">
          {name}
          {props.optional === true ? (
            <span className="text-xs font-normal text-fg-subtle">{permissionsCopy.optional}</span>
          ) : null}
        </span>
        <span className="text-sm text-fg-muted">{why}</span>
      </div>
      <RowControl state={state} {...props} />
    </li>
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
