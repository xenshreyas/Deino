/**
 * "Jump to now" (ui.md Transcript bubble): while a live transcript is
 * scrolled up, a 32px ink pill at the bottom centre brings the newest line
 * back and resumes following it.
 */
import { cn } from '@/ui/cn';
import { ArrowDown, ICON_PROPS } from '@/ui/icons';

import { transcriptCopy } from '../copy';

/** Props of {@link JumpToNow}. */
export interface JumpToNowProps {
  visible: boolean;
  onJump: () => void;
}

/** The pill; it fades and lifts in, and leaves the tab order while hidden. */
export function JumpToNow({ visible, onJump }: JumpToNowProps) {
  return (
    <button
      type="button"
      onClick={onJump}
      tabIndex={visible ? 0 : -1}
      aria-hidden={!visible}
      className={cn(
        'absolute bottom-4 left-1/2 z-(--z-sticky) flex h-8 -translate-x-1/2 items-center gap-1.5 rounded-full bg-fg px-3 text-xs font-semibold text-on-fg shadow-popover',
        'transition-[opacity,translate] duration-(--dur-base) ease-out focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
        visible ? 'translate-y-0 opacity-100' : 'pointer-events-none translate-y-2 opacity-0',
      )}
    >
      <ArrowDown {...ICON_PROPS} size={12} />
      {transcriptCopy.jumpToNow}
    </button>
  );
}
