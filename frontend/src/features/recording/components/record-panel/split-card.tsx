/**
 * "Start a new note?" (CAP-07, back-to-back meetings): a small card under
 * the pill when the next calendar meeting begins with other people while
 * this one records. It never interrupts: it waits, and goes on its own
 * when the moment passes. Presentational.
 */
import { Button } from '@/ui/button';

import { panelCopy } from '../../copy';
import type { SplitOffer } from '../../types';

/** Props of {@link SplitCard}. */
export interface SplitCardProps {
  offer: SplitOffer;
  busy: boolean;
  onAccept: () => void;
  onDismiss: () => void;
}

/** The offer, with its two answers. */
export function SplitCard({ offer, busy, onAccept, onDismiss }: SplitCardProps) {
  const title = offer.title.trim() === '' ? panelCopy.untitledEvent : offer.title;
  return (
    <section
      aria-label={panelCopy.splitBody}
      className="flex w-[300px] animate-in flex-col gap-2.5 rounded-xl bg-surface-raised/95 px-3.5 py-3 shadow-pill backdrop-blur-xl duration-(--dur-slow) ease-out fade-in-0 slide-in-from-top-1"
    >
      <div className="flex min-w-0 flex-col">
        <p className="truncate text-xs font-semibold text-fg" title={title}>
          {panelCopy.splitTitle(title)}
        </p>
        <p className="text-xs text-fg-muted">{panelCopy.splitBody}</p>
      </div>
      <div className="flex justify-end gap-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2.5 text-xs"
          disabled={busy}
          onClick={onDismiss}
        >
          {panelCopy.splitDismiss}
        </Button>
        <Button
          variant="primary"
          size="sm"
          className="h-7 px-2.5 text-xs"
          disabled={busy}
          onClick={onAccept}
        >
          {panelCopy.splitAccept}
        </Button>
      </div>
    </section>
  );
}
