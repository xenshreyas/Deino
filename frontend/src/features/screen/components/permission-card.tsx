/**
 * The Screen Recording row: its state, and the way to System Settings when
 * it is missing. Deino never shows the macOS prompt from here; the status
 * is polled, so the row turns Allowed when the user comes back.
 * Presentational.
 */
import { Button } from '@/ui/button';
import { cn } from '@/ui/cn';
import { SettingsCard } from '@/ui/settings-section';

import { screenCopy } from '../copy';

const copy = screenCopy.permission;

/** Props of {@link PermissionCard}. */
export interface PermissionCardProps {
  /** This Mac can capture the screen at all. */
  supported: boolean;
  /** Screen Recording is granted. */
  permitted: boolean;
  /** System Settings is being opened. */
  opening: boolean;
  /** Opening System Settings failed. */
  openFailed: boolean;
  onOpenSettings: () => void;
}

function StateLabel({ permitted }: { permitted: boolean }) {
  return (
    <span className="inline-flex items-center gap-1.5 text-xs">
      <span
        aria-hidden
        className={cn('size-1.5 rounded-full', permitted ? 'bg-success' : 'bg-warning')}
      />
      <span className={permitted ? 'text-fg-muted' : 'text-warning-text'}>
        {permitted ? copy.granted : copy.missing}
      </span>
    </span>
  );
}

/** Screen Recording's state and its System Settings button. */
export function PermissionCard({
  supported,
  permitted,
  opening,
  openFailed,
  onOpenSettings,
}: PermissionCardProps) {
  return (
    <>
      <SettingsCard label={copy.title}>
        <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
          <div className="min-w-0">
            <p className="text-sm text-fg">{copy.name}</p>
            <p className="mt-0.5 text-xs text-fg-muted">
              {supported ? copy.why : copy.unsupported}
            </p>
          </div>
          {supported ? (
            <div className="flex items-center gap-3">
              <StateLabel permitted={permitted} />
              {permitted ? null : (
                <Button variant="secondary" size="sm" disabled={opening} onClick={onOpenSettings}>
                  {copy.openSettings}
                </Button>
              )}
            </div>
          ) : null}
        </div>
      </SettingsCard>
      {supported && !permitted ? <p className="px-1 text-xs text-fg-muted">{copy.hint}</p> : null}
      {openFailed ? (
        <p role="alert" className="px-1 text-xs text-danger">
          {copy.settingsFailed}
        </p>
      ) : null}
    </>
  );
}
