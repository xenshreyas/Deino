/**
 * The Notes tab: the meeting's template (NOT-04), its action items (AI-05),
 * then the AI summary and
 * "My notes" behind a segmented pill (ui.md §3 Tabs, "Notes sub-switch");
 * ⌘E switches between them. NOT-01
 * fills "My notes" with the notepad; NOT-02 replaces the summary with the
 * enhanced notes. A meeting without a summary opens on "My notes".
 */
import { useState } from 'react';

import { MeetingActionItems } from '@/features/action-items';
import { Notepad, notepadCopy } from '@/features/notepad';
import { MeetingTemplatePicker } from '@/features/templates';
import { useMountEffect } from '@/hooks/useMountEffect';
import { SegmentedControl } from '@/ui/segmented-control';

import { useLegacyMeetingPanels } from '../legacy/panels-context';
import type { MeetingSlotProps } from '../types';
import { LegacyNotesPanel } from './legacy-slots';

/** The Notes tab's two views. */
type NotesView = 'summary' | 'mine';

const VIEWS = [
  { value: 'summary', label: notepadCopy.summary },
  { value: 'mine', label: notepadCopy.myNotes },
] as const;

function other(view: NotesView): NotesView {
  return view === 'mine' ? 'summary' : 'mine';
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

/** Notes: the summary or the user's own notes. */
export function NotesSlot({ meetingId }: MeetingSlotProps) {
  const panels = useLegacyMeetingPanels();
  const [chosen, setChosen] = useState<NotesView | null>(null);
  // ⌘E presses since the last click; an odd count shows the other view.
  const [flips, setFlips] = useState(0);
  const fallback: NotesView = panels.status === 'ready' && panels.hasSummary ? 'summary' : 'mine';
  const base = chosen ?? fallback;
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
      <MeetingTemplatePicker
        meetingId={meetingId}
        onRegenerate={panels.status === 'ready' ? panels.regenerateNotes : null}
      />
      <MeetingActionItems meetingId={meetingId} />
      {view === 'mine' ? (
        <Notepad meetingId={meetingId} toolbar={toggle} />
      ) : (
        <>
          <div className="flex h-9 shrink-0 items-center justify-end">{toggle}</div>
          <LegacyNotesPanel />
        </>
      )}
    </div>
  );
}
