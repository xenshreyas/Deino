/**
 * The Enhanced view's resting states (ui.md §1 rule 9): the loading shape,
 * notes being written, no notes yet (one action, its shortcut), and a
 * failure that says what to do.
 */
import { Link } from '@tanstack/react-router';

import { IpcError } from '@/lib/ipc/result';
import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { CircleAlert, ICON_PROPS, Sparkles } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';
import { Skeleton, SkeletonText } from '@/ui/skeleton';

import { notesCopy } from '../copy';
import { ENHANCE_SHORTCUT } from './enhance-button';

/** The notes' loading shape: a heading and a few lines, twice. */
export function NotesSkeleton() {
  return (
    <div className="flex flex-col gap-4 pt-4" role="status" aria-label="Loading">
      <Skeleton className="h-3.5 w-28" />
      <SkeletonText lines={3} />
      <Skeleton className="mt-2 h-3.5 w-36" />
      <SkeletonText lines={2} />
    </div>
  );
}

/** Notes are being written: a calm line above the shape they will take. */
export function WritingNotes() {
  return (
    <div className="flex flex-col gap-5 pt-4" role="status" aria-live="polite">
      <p className="flex items-center gap-2 text-sm text-ai-text">
        <Sparkles {...ICON_PROPS} className="animate-pulse text-accent" />
        {notesCopy.writing}
      </p>
      <div className="flex flex-col gap-4" aria-hidden>
        <Skeleton className="h-3.5 w-28" />
        <SkeletonText lines={4} />
        <Skeleton className="mt-2 h-3.5 w-40" />
        <SkeletonText lines={3} />
      </div>
    </div>
  );
}

/** No notes yet: what Enhance does, and the button. */
export function NoNotesYet({
  onEnhance,
  blocked,
  error,
}: {
  onEnhance: () => void;
  /** Why notes cannot be written (PRV-01: the transcript was deleted), or `null`. */
  blocked: string | null;
  error: Error | null;
}) {
  return (
    <div className="flex flex-col items-center">
      <EmptyState
        icon={Sparkles}
        title={notesCopy.emptyTitle}
        description={blocked ?? notesCopy.emptyBody}
        className="pt-[14vh]"
        action={
          <Button variant="secondary" onClick={onEnhance} disabled={blocked !== null}>
            <Sparkles {...ICON_PROPS} />
            {notesCopy.enhance}
            <Kbd className="ml-1">{ENHANCE_SHORTCUT}</Kbd>
          </Button>
        }
      />
      {error === null ? null : <NotesFailure error={error} className="mt-6 max-w-md" />}
    </div>
  );
}

/** Why notes could not be written, with the way to fix it. */
export function NotesFailure({ error, className }: { error: Error; className?: string }) {
  const needsSetup = error instanceof IpcError && error.code === 'unavailable';
  return (
    <Alert variant="destructive" className={className}>
      <CircleAlert {...ICON_PROPS} />
      <AlertDescription className="flex flex-col gap-1">
        <span className="font-semibold">{notesCopy.failedTitle}</span>
        <span>{error.message}</span>
        {needsSetup ? (
          <Link to="/settings" search={{ section: 'ai' }} className="underline underline-offset-4">
            {notesCopy.openSettings}
          </Link>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
