/**
 * One meeting app's row in Settings → Speaker tags: the app, where it runs,
 * its state (Enabled, Needs permission, Unsupported), and a way to System
 * Settings when permission is missing. Presentational.
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';

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

/** Props of {@link AppTagRow}. */
export interface AppTagRowProps {
  status: AppTagStatus;
  /** System Settings is being opened. */
  opening: boolean;
  onOpenSettings: () => void;
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

/** One meeting app. */
export function AppTagRow({ status, opening, onOpenSettings }: AppTagRowProps) {
  const app = copy.apps[status.app];
  const needsPermission = status.state === 'needsPermission';
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-sm text-fg">{app.name}</p>
        <p className="mt-0.5 line-clamp-2 text-xs text-fg-muted">
          {status.state === 'unsupported' ? copy.unsupportedHint : app.where}
        </p>
      </div>
      <div className="flex items-center gap-3">
        <StateLabel state={status.state} />
        {needsPermission ? (
          <Button
            variant="secondary"
            size="sm"
            disabled={opening}
            aria-label={copy.openSettingsLabel(app.name)}
            onClick={onOpenSettings}
          >
            {copy.openSettings}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
