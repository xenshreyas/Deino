/**
 * The consent reminder (CAP-13): a small card under the recording pill when
 * a recording starts, in the same shape as auto-stop's split card. It asks
 * nothing of the recording: "Got it" hides it for this meeting, "Don't show
 * again" turns it off in Settings → Consent. Presentational.
 */
import { Button } from '@/ui/button';

import { consentCopy } from '../copy';

const copy = consentCopy.reminder;

/** Props of {@link ConsentCard}. */
export interface ConsentCardProps {
  busy: boolean;
  onDismiss: () => void;
  onNever: () => void;
}

/** The reminder, with its two answers. */
export function ConsentCard({ busy, onDismiss, onNever }: ConsentCardProps) {
  return (
    <section
      aria-label={copy.label}
      className="flex w-[300px] animate-in flex-col gap-2.5 rounded-xl bg-surface-raised/95 px-3.5 py-3 shadow-pill backdrop-blur-xl duration-(--dur-slow) ease-out fade-in-0 slide-in-from-top-1"
    >
      <div className="flex min-w-0 flex-col">
        <p className="text-xs font-semibold text-fg">{copy.title}</p>
        <p className="text-xs text-fg-muted">{copy.body}</p>
      </div>
      <div className="flex justify-end gap-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2.5 text-xs"
          disabled={busy}
          onClick={onNever}
        >
          {copy.never}
        </Button>
        <Button
          variant="primary"
          size="sm"
          className="h-7 px-2.5 text-xs"
          disabled={busy}
          onClick={onDismiss}
        >
          {copy.dismiss}
        </Button>
      </div>
    </section>
  );
}
