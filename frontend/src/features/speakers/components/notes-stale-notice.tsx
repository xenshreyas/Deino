/**
 * "Names changed. Regenerate the notes to use them." Shown in the panel
 * after a speaker is named for all segments, merged or confirmed, because
 * the notes were written with the old names (they are regenerated on
 * request, never behind the user's back).
 */
import { Button } from '@/ui/button';
import { IconButton } from '@/ui/icon-button';
import { ICON_PROPS, RotateCcw, X } from '@/ui/icons';

import { speakersCopy } from '../copy';

/** Props of {@link NotesStaleNotice}. */
export interface NotesStaleNoticeProps {
  onRegenerate: () => void;
  onDismiss: () => void;
  regenerating: boolean;
}

/** The inline notice with its one action. */
export function NotesStaleNotice({ onRegenerate, onDismiss, regenerating }: NotesStaleNoticeProps) {
  return (
    <div
      role="status"
      className="mx-2 mb-2 flex flex-col gap-2 rounded-lg border border-border bg-bg-subtle p-3"
    >
      <div className="flex items-start gap-2">
        <p className="flex-1 text-xs text-pretty text-fg">{speakersCopy.notesStale}</p>
        <IconButton
          icon={X}
          label={speakersCopy.dismissNotice}
          className="-mt-1 -mr-1"
          onClick={onDismiss}
        />
      </div>
      <Button
        size="sm"
        variant="secondary"
        className="self-start"
        disabled={regenerating}
        onClick={onRegenerate}
      >
        <RotateCcw {...ICON_PROPS} />
        {speakersCopy.regenerate}
      </Button>
    </div>
  );
}
