/**
 * The meeting header's Enhance button (ui.md §3 Meeting header: ghost sm
 * with a sparkles icon), ⌘⇧E anywhere on the meeting page. It reads
 * "Regenerate" once notes exist, shows a spinner while notes are being
 * written (here or by the pipeline), and hosts the dialog that asks before
 * replacing notes the user edited. Once retention deleted the transcript
 * (PRV-01) it is disabled, and its hint says why.
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

/** The element that says why the button is disabled. */
const BLOCKED_ID = 'enhance-blocked-reason';

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

/** Props of {@link Trigger}. */
interface TriggerProps {
  button: RefObject<HTMLButtonElement>;
  label: string;
  hint: string;
  busy: boolean;
  /** Why it is disabled, or `null`. */
  blocked: string | null;
  onClick: () => void;
}

/**
 * The button and its hint. A disabled button gets no pointer events, so a
 * blocked one sits in a focusable span that keeps the hint, and names the
 * reason for screen readers.
 */
function Trigger({ button, label, hint, busy, blocked, onClick }: TriggerProps) {
  return (
    <>
      <Hint label={blocked ?? hint} shortcut={blocked === null ? ENHANCE_SHORTCUT : undefined}>
        <span className="inline-flex" tabIndex={blocked === null ? undefined : 0}>
          <Button
            ref={button}
            variant="ghost"
            size="sm"
            onClick={onClick}
            disabled={busy || blocked !== null}
            aria-busy={busy}
            aria-describedby={blocked === null ? undefined : BLOCKED_ID}
          >
            {busy ? (
              <LoaderCircle {...ICON_PROPS} className="animate-spin" />
            ) : (
              <Sparkles {...ICON_PROPS} />
            )}
            {label}
          </Button>
        </span>
      </Hint>
      {blocked === null ? null : (
        <span id={BLOCKED_ID} className="sr-only">
          {blocked}
        </span>
      )}
    </>
  );
}

/** Enhance (or Regenerate) the meeting's notes. */
export function EnhanceButton({ meetingId }: EnhanceButtonProps) {
  const action = useEnhanceAction(meetingId);
  const notes = useEnhancedNotes(meetingId);
  const busy = useIsEnhancing(meetingId);
  const hasNotes = notes.data !== null && notes.data !== undefined;
  const button = useRef<HTMLButtonElement>(null);
  useEnhanceShortcut(button);
  return (
    <>
      <Trigger
        button={button}
        label={hasNotes ? notesCopy.regenerate : notesCopy.enhance}
        hint={hasNotes ? notesCopy.regenerateHint : notesCopy.enhanceHint}
        busy={busy}
        blocked={action.blocked}
        onClick={action.run}
      />
      <ConfirmRegenerate meetingId={meetingId} onConfirm={action.replace} />
    </>
  );
}
