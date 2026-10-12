/**
 * "Transcript changed: Regenerate notes?" (TRN-02, Granola §4): after a
 * line is edited, deleted or replaced, a quiet line above the transcript
 * offers to rewrite the notes, so deleted words leave them too. It shows
 * exactly when the newest notes' regenerate reasons (NOT-02's typed
 * `RegenerateReason`s, ADR 0030) say the transcript was edited since they
 * were written, and Not now hides it until the notes change.
 */
import { useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { useEnhanceAction, useEnhancedNotes, useIsEnhancing } from '@/features/notes';
import { Button } from '@/ui/button';
import { ICON_PROPS, RotateCcw } from '@/ui/icons';

import { transcriptCopy } from '../copy';

/** The banner; nothing while the notes are current. */
export function TranscriptChangedBanner({ meetingId }: { meetingId: MeetingId }) {
  const notes = useEnhancedNotes(meetingId);
  const enhance = useEnhanceAction(meetingId);
  const busy = useIsEnhancing(meetingId);
  const [dismissed, setDismissed] = useState<number | null>(null);
  const view = notes.data;
  if (view?.isLatest !== true || busy) return null;
  if (!view.reasons.includes('transcriptEdited') || dismissed === view.version.version) return null;
  return (
    <div
      role="status"
      className="mb-2 flex shrink-0 items-center gap-3 rounded-lg bg-accent-soft px-3 py-2 text-sm text-fg"
    >
      <RotateCcw {...ICON_PROPS} className="shrink-0 text-accent" />
      <span className="min-w-0 flex-1 text-pretty">
        <span className="font-semibold">{transcriptCopy.changedLead}:</span>{' '}
        {transcriptCopy.changedQuestion}
      </span>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          setDismissed(view.version.version);
        }}
      >
        {transcriptCopy.notNow}
      </Button>
      <Button size="sm" onClick={enhance.run}>
        {transcriptCopy.regenerate}
      </Button>
    </div>
  );
}
