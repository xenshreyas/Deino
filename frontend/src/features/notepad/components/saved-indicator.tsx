/**
 * The autosave's quiet status: nothing while typing, "Saving…" and then
 * "Saved" in the subtlest text, and a plain warning only when a save failed.
 */
import { cn } from '@/ui/cn';
import { Check, ICON_PROPS } from '@/ui/icons';

import { notepadCopy } from '../copy';
import type { SaveStatus } from '../types';

const LABELS: Readonly<Record<SaveStatus, string>> = {
  idle: '',
  pending: '',
  saving: notepadCopy.saving,
  saved: notepadCopy.saved,
  error: notepadCopy.notSaved,
};

/** The status line; announced politely to screen readers. */
export function SavedIndicator({ status }: { status: SaveStatus }) {
  const label = LABELS[status];
  return (
    <span
      role="status"
      aria-live="polite"
      className={cn(
        'inline-flex h-5 items-center gap-1 text-xs tabular-nums transition-opacity duration-(--dur-base) ease-(--ease-out)',
        status === 'error' ? 'text-danger' : 'text-fg-subtle',
        label === '' ? 'opacity-0' : 'opacity-100',
      )}
    >
      {status === 'saved' ? <Check {...ICON_PROPS} size={12} /> : null}
      {label}
    </span>
  );
}
