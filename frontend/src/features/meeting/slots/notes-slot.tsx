/**
 * The Notes tab (UX-07: the notes start near the top): the pre-meeting
 * brief of its calendar event (NOT-08) only before the event ends or while
 * the meeting records, its action items (AI-05), then the enhanced notes
 * (NOT-02) or "My notes" (NOT-01) behind a segmented pill (ui.md §3 Tabs,
 * "Notes sub-switch") with the template menu (NOT-04) beside it; ⌘E
 * switches between them. Insight answers (AI-06) and the shared screen's
 * images (CAP-09) follow the notes, when there are any. The tab opens on
 * Enhanced when the meeting has enhanced notes or they are being written,
 * else on My notes. Switching the template offers to regenerate the notes
 * (Granola §3.3). The follow-up email is in the context rail, or Details.
 */
import { useState } from 'react';

import { MeetingActionItems } from '@/features/action-items';
import { MeetingBrief } from '@/features/briefs';
import { CommentLayer } from '@/features/comments';
import { MeetingInsights } from '@/features/insights';
import {
  EnhancedNotes,
  notesCopy,
  useEnhanceAction,
  useEnhancedNotes,
  useIsEnhancing,
} from '@/features/notes';
import { Notepad, notepadCopy } from '@/features/notepad';
import { MeetingImages } from '@/features/screen';
import { MeetingTemplateMenu, TemplateSwitchedPrompt } from '@/features/templates';
import { useMountEffect } from '@/hooks/useMountEffect';
import { SegmentedControl } from '@/ui/segmented-control';

import { useBriefShown } from '../hooks/use-is-live';
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

/** Which view shows: the user's pick, else Enhanced when there are notes; ⌘E flips it. */
function useNotesView(meetingId: MeetingSlotProps['meetingId']) {
  const enhanced = useEnhancedNotes(meetingId);
  const busy = useIsEnhancing(meetingId);
  const [chosen, setChosen] = useState<NotesView | null>(null);
  // ⌘E presses since the last click; an odd count shows the other view.
  const [flips, setFlips] = useState(0);
  const hasNotes = enhanced.data !== null && enhanced.data !== undefined;
  const base = chosen ?? (hasNotes || busy ? 'enhanced' : 'mine');
  useToggleShortcut(() => {
    setFlips((count) => count + 1);
  });
  const choose = (next: NotesView) => {
    setChosen(next);
    setFlips(0);
  };
  return { view: flips % 2 === 0 ? base : other(base), choose, hasNotes };
}

/** The status row's right side: the template menu, then the view switch. */
function Toolbar(props: {
  meetingId: MeetingSlotProps['meetingId'];
  view: NotesView;
  onView: (view: NotesView) => void;
  onSwitched: (name: string) => void;
}) {
  return (
    <div className="flex items-center gap-1">
      <MeetingTemplateMenu meetingId={props.meetingId} onSwitched={props.onSwitched} />
      <SegmentedControl
        label={notepadCopy.viewLabel}
        segments={VIEWS}
        value={props.view}
        onValueChange={props.onView}
      />
    </div>
  );
}

/** After a template switch: regenerate the notes written with the old one? */
function SwitchedPrompt(props: {
  meetingId: MeetingSlotProps['meetingId'];
  name: string | null;
  hasNotes: boolean;
  onDone: () => void;
}) {
  const regenerate = useEnhanceAction(props.meetingId);
  if (props.name === null || !props.hasNotes || regenerate.blocked !== null) return null;
  return (
    <TemplateSwitchedPrompt
      name={props.name}
      onRegenerate={() => {
        props.onDone();
        regenerate.run();
      }}
      onDismiss={props.onDone}
    />
  );
}

/** What follows the notes: insight answers and the screen's images, when there are any. */
function Footer({ meetingId }: MeetingSlotProps) {
  return (
    <div className="flex flex-col gap-8 pt-10">
      <MeetingInsights meetingId={meetingId} />
      <MeetingImages meetingId={meetingId} />
    </div>
  );
}

/** Notes: the enhanced notes or the user's own. */
export function NotesSlot({ meetingId }: MeetingSlotProps) {
  const { view, choose, hasNotes } = useNotesView(meetingId);
  const [switched, setSwitched] = useState<string | null>(null);
  const briefShown = useBriefShown(meetingId);
  const toolbar = (
    <Toolbar meetingId={meetingId} view={view} onView={choose} onSwitched={setSwitched} />
  );
  const footer = <Footer meetingId={meetingId} />;
  return (
    <div className="flex h-full min-h-0 flex-col">
      {briefShown ? <MeetingBrief meetingId={meetingId} /> : null}
      <MeetingActionItems meetingId={meetingId} />
      <SwitchedPrompt
        meetingId={meetingId}
        name={switched}
        hasNotes={hasNotes}
        onDone={() => {
          setSwitched(null);
        }}
      />
      <CommentLayer
        key={view}
        meetingId={meetingId}
        target={view === 'mine' ? 'notes' : 'enhanced'}
        className="flex min-h-0 flex-1 flex-col"
      >
        {view === 'mine' ? (
          <Notepad meetingId={meetingId} toolbar={toolbar} footer={footer} />
        ) : (
          <EnhancedNotes meetingId={meetingId} toolbar={toolbar} footer={footer} />
        )}
      </CommentLayer>
    </div>
  );
}
