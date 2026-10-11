/**
 * The regenerate prompt (Granola §3.3): a quiet line above the notes when
 * something they were written from changed since (the calendar event,
 * attendees, the transcript, the user's notes, more audio), saying what, with Regenerate
 * and Not now.
 */
import { Button } from '@/ui/button';
import { ICON_PROPS, RotateCcw } from '@/ui/icons';

import { notesCopy, reasonsText } from '../copy';
import type { RegenerateReason } from '../types';

/** Props of {@link RegenerateBanner}. */
export interface RegenerateBannerProps {
  reasons: readonly RegenerateReason[];
  onRegenerate: () => void;
  onDismiss: () => void;
}

/** The banner; nothing when nothing changed. */
export function RegenerateBanner({ reasons, onRegenerate, onDismiss }: RegenerateBannerProps) {
  if (reasons.length === 0) return null;
  const why = reasonsText(reasons);
  return (
    <div
      role="status"
      className="mb-3 flex items-center gap-3 rounded-lg bg-accent-soft px-3 py-2 text-sm text-fg"
    >
      <RotateCcw {...ICON_PROPS} className="shrink-0 text-accent" />
      <span className="min-w-0 flex-1 text-pretty">
        {notesCopy.bannerLead}: {why}.
      </span>
      <Button variant="ghost" size="sm" onClick={onDismiss}>
        {notesCopy.dismiss}
      </Button>
      <Button size="sm" onClick={onRegenerate}>
        {notesCopy.regenerate}
      </Button>
    </div>
  );
}
