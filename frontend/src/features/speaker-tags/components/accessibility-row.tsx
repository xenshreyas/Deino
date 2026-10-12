/**
 * The one Accessibility row of Settings → Recording → Speaker names (UX-09):
 * one macOS permission, one row, one button while it is missing. The meeting
 * apps follow as plain text, marked only when one is unsupported here.
 * Presentational.
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { SettingsRow } from '@/ui/settings-section';

import { speakerTagsCopy as copy } from '../copy';
import type { AppTagStatus, TagState } from '../types';

/** The dot beside each state: calm, warning, or quiet. */
const DOT: Record<TagState, string> = {
  enabled: 'bg-success',
  needsPermission: 'bg-warning',
  unsupported: 'bg-fg-subtle',
};

/** The state's words. */
const TEXT: Record<TagState, string> = {
  enabled: 'text-fg-muted',
  needsPermission: 'text-warning-text',
  unsupported: 'text-fg-subtle',
};

/**
 * The permission's state across the apps: missing when any app needs it,
 * allowed when any app can read names, otherwise not available here.
 */
export function permissionState(statuses: readonly AppTagStatus[]): TagState {
  if (statuses.some((status) => status.state === 'needsPermission')) return 'needsPermission';
  if (statuses.some((status) => status.state === 'enabled')) return 'enabled';
  return 'unsupported';
}

/** "Works with Zoom, Microsoft Teams and Google Meet (not supported here)." */
export function worksWith(statuses: readonly AppTagStatus[]): string {
  const names = statuses.map((status) => {
    const name = copy.apps[status.app].name;
    return status.state === 'unsupported' ? copy.unsupportedApp(name) : name;
  });
  const last = names.pop();
  if (last === undefined) return '';
  return copy.worksWith(names.length === 0 ? last : `${names.join(', ')} and ${last}`);
}

/** The state's dot and word. */
function StateLabel({ state }: { state: TagState }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-xs">
      <span aria-hidden className={cn('size-1.5 rounded-full', DOT[state])} />
      <span className={TEXT[state]}>{copy.states[state]}</span>
    </span>
  );
}

/** Props of {@link AccessibilityRow}. */
export interface AccessibilityRowProps {
  state: TagState;
  /** System Settings is being opened. */
  opening: boolean;
  onOpenSettings: () => void;
}

/** Accessibility: its state, and the way to System Settings while it is missing. */
export function AccessibilityRow({ state, opening, onOpenSettings }: AccessibilityRowProps) {
  return (
    <SettingsRow label={copy.permission.name} description={copy.permission.why}>
      <div className="flex items-center gap-3">
        <StateLabel state={state} />
        {state === 'needsPermission' ? (
          <Button
            variant="secondary"
            size="sm"
            disabled={opening}
            aria-label={copy.openSettingsLabel}
            onClick={onOpenSettings}
          >
            {copy.openSettings}
          </Button>
        ) : null}
      </div>
    </SettingsRow>
  );
}
