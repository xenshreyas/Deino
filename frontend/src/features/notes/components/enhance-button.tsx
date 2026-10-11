/**
 * The meeting header's Enhance button (ui.md §3 Meeting header: ghost sm
 * with a sparkles icon), ⌘⇧E anywhere on the meeting page. It reads
 * "Regenerate" once notes exist, shows a spinner while notes are being
 * written (here or by the pipeline), and hosts the dialog that asks before
 * replacing notes the user edited.
 */
import { type RefObject, useRef } from 'react';

import type { MeetingId } from '@/features/meetings';
import { useMountEffect } from '@/hooks/useMountEffect';
import { Button } from '@/ui/button';
import { ICON_PROPS, LoaderCircle, Sparkles } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { notesCopy } from '../copy';
import { useEnhanceAction } from '../hooks/use-enhance-action';
import { useEnhancedNotes, useIsEnhancing } from '../queries';
import { ConfirmRegenerate } from './confirm-regenerate';

/** The shortcut, shown in the tooltip. */
export const ENHANCE_SHORTCUT = '⌘⇧E';

/**
 * ⌘⇧E presses `button` while the meeting page is open; a disabled button
 * (notes already being written) ignores it.
 */
function useEnhanceShortcut(button: RefObject<HTMLButtonElement | null>): void {
  useMountEffect(() => {
    const listener = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey;
      if (!mod || !event.shiftKey || event.altKey || event.key.toLowerCase() !== 'e') return;
      event.preventDefault();
      button.current?.click();
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
}

/** Props of {@link EnhanceButton}. */
export interface EnhanceButtonProps {
  meetingId: MeetingId;
}

/** Enhance (or Regenerate) the meeting's notes. */
export function EnhanceButton({ meetingId }: EnhanceButtonProps) {
  const action = useEnhanceAction(meetingId);
  const notes = useEnhancedNotes(meetingId);
  const busy = useIsEnhancing(meetingId);
  const hasNotes = notes.data !== null && notes.data !== undefined;
  const button = useRef<HTMLButtonElement>(null);
  useEnhanceShortcut(button);
  const label = hasNotes ? notesCopy.regenerate : notesCopy.enhance;
  return (
    <>
      <Hint
        label={hasNotes ? notesCopy.regenerateHint : notesCopy.enhanceHint}
        shortcut={ENHANCE_SHORTCUT}
      >
        <Button
          ref={button}
          variant="ghost"
          size="sm"
          onClick={action.run}
          disabled={busy}
          aria-busy={busy}
        >
          {busy ? (
            <LoaderCircle {...ICON_PROPS} className="animate-spin" />
          ) : (
            <Sparkles {...ICON_PROPS} />
          )}
          {label}
        </Button>
      </Hint>
      <ConfirmRegenerate meetingId={meetingId} onConfirm={action.replace} />
    </>
  );
}
