import { ICON_PROPS, Play } from '@/ui/icons';

import { meetingCopy } from '../copy';

/**
 * The bottom player slot, resting until TRN-03 adds synced playback: a quiet
 * bar with a disabled play control and a 0:00 scrubber.
 */
export function PlayerPlaceholder() {
  return (
    <div
      aria-label={meetingCopy.playerLabel}
      role="group"
      className="flex h-11 items-center gap-3 border-t border-border px-4 text-xs text-fg-subtle"
    >
      <button
        type="button"
        disabled
        aria-label={meetingCopy.playerSoon}
        className="flex size-7 items-center justify-center rounded-full bg-bg-muted text-fg-subtle"
      >
        <Play {...ICON_PROPS} size={14} />
      </button>
      <span className="tabular-nums">0:00</span>
      <div className="h-1 flex-1 rounded-full bg-bg-muted" />
      <span>{meetingCopy.playerSoon}</span>
    </div>
  );
}
