import { ICON_PROPS, LoaderCircle } from '@/ui/icons';

/** Props of {@link StatusOverlays}. */
interface StatusOverlaysProps {
  /** Finishing the transcription after the recording stopped. */
  isProcessing: boolean;
  /** Saving the transcript to the database. */
  isSaving: boolean;
}

/** One floating status pill at the bottom centre of the record screen. */
function StatusPill({ message }: { message: string }) {
  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-6 z-(--z-chatbar) flex justify-center">
      <div
        role="status"
        className="flex h-9 items-center gap-2 rounded-full bg-surface-raised px-4 text-sm text-fg shadow-pill"
      >
        <LoaderCircle {...ICON_PROPS} className="animate-spin text-fg-muted" />
        {message}
      </div>
    </div>
  );
}

/** The "Finalizing…" and "Saving…" pills shown after Stop. */
export function StatusOverlays({ isProcessing, isSaving }: StatusOverlaysProps) {
  if (isSaving) return <StatusPill message="Saving transcript…" />;
  if (isProcessing) return <StatusPill message="Finalizing transcription…" />;
  return null;
}
