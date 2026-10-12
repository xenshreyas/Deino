/**
 * The Notes tab: the pre-meeting brief of its calendar event (NOT-08), the
 * suggested follow-up email's "View draft" banner (NOT-07), the Images
 * strip of the shared screen's snapshots (CAP-09), the
 * meeting's template (NOT-04), its action items (AI-05),
 * then the enhanced notes (NOT-02) and "My notes" (NOT-01) behind a
 * segmented pill (ui.md §3 Tabs, "Notes sub-switch"); ⌘E switches between
 * them. The tab opens on Enhanced when the meeting has enhanced notes or
 * they are being written, else on My notes. Switching the template
 * regenerates the notes (Granola §3.3).
 */
import { useState } from 'react';

import { MeetingActionItems } from '@/features/action-items';
import { CommentLayer } from '@/features/comments';
import { MeetingBrief } from '@/features/briefs';
import { MeetingFollowUp } from '@/features/followups';
import {
  EnhancedNotes,
  notesCopy,
  useEnhanceAction,
  useEnhancedNotes,
  useIsEnhancing,
} from '@/features/notes';
import { Notepad, notepadCopy } from '@/features/notepad';
import { MeetingImages } from '@/features/screen';
import { MeetingTemplatePicker } from '@/features/templates';
import { useMountEffect } from '@/hooks/useMountEffect';
import { SegmentedControl } from '@/ui/segmented-control';

import type { MeetingSlotProps } from '../types';

/** The Notes tab's two views. */
type NotesView = 'enhanced' | 'mine';

const VIEWS = [
  { value: 'enhanced', label: notesCopy.enhanced },
  { value: 'mine', label: notepadCopy.myNotes },
] as const;

function other(view: NotesView): NotesView {
  return view === 'mine' ? 'enhanced' : 'mine';
}

/** ⌘E toggles the view while the meeting page is open. */
function useToggleShortcut(toggle: () => void): void {
  useMountEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.shiftKey || event.key !== 'e') return;
      event.preventDefault();
      toggle();
    };
    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  });
}

/** The pill switch. */
function ViewSwitch({ view, onChange }: { view: NotesView; onChange: (view: NotesView) => void }) {
  return (
    <SegmentedControl
      label={notepadCopy.viewLabel}
      segments={VIEWS}
      value={view}
      onValueChange={onChange}
    />
  );
}

/** What comes before the notes: the brief (NOT-08) and the follow-up banner (NOT-07). */
function Preface({ meetingId }: MeetingSlotProps) {
  return (
    <>
      <MeetingBrief meetingId={meetingId} />
      <MeetingFollowUp meetingId={meetingId} />
      <MeetingImages meetingId={meetingId} />
    </>
  );
}

/** Notes: the enhanced notes or the user's own. */
export function NotesSlot({ meetingId }: MeetingSlotProps) {
  const enhanced = useEnhancedNotes(meetingId);
  const busy = useIsEnhancing(meetingId);
  const regenerate = useEnhanceAction(meetingId);
  const [chosen, setChosen] = useState<NotesView | null>(null);
  // ⌘E presses since the last click; an odd count shows the other view.
  const [flips, setFlips] = useState(0);
  const hasNotes = enhanced.data !== null && enhanced.data !== undefined;
  const base = chosen ?? (hasNotes || busy ? 'enhanced' : 'mine');
  const view = flips % 2 === 0 ? base : other(base);
  useToggleShortcut(() => {
    setFlips((count) => count + 1);
  });
  const choose = (next: NotesView) => {
    setChosen(next);
    setFlips(0);
  };
  const toggle = <ViewSwitch view={view} onChange={choose} />;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <Preface meetingId={meetingId} />
      <MeetingTemplatePicker
        meetingId={meetingId}
        onRegenerate={hasNotes && regenerate.blocked === null ? regenerate.run : null}
      />
      <MeetingActionItems meetingId={meetingId} />
      <CommentLayer
        key={view}
        meetingId={meetingId}
        target={view === 'mine' ? 'notes' : 'enhanced'}
        className="flex min-h-0 flex-1 flex-col"
      >
        {view === 'mine' ? (
          <Notepad meetingId={meetingId} toolbar={toggle} />
        ) : (
          <EnhancedNotes meetingId={meetingId} toolbar={toggle} />
        )}
      </CommentLayer>
    </div>
  );
}
