/**
 * The record screen's notepad. It writes before a meeting row exists: the
 * first edit creates a scheduled draft (`live-notes.ts`), later edits
 * autosave into it, and stopping a recording moves the notes into the
 * saved meeting. Coming back to the screen reopens the same draft.
 */
import { useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { useNativeImageDrop } from '../hooks/use-native-image-drop';
import { useNoteEditor } from '../hooks/use-note-editor';
import { liveTarget, useLiveNotes } from '../live-notes';
import { useMeetingNotes } from '../queries';
import type { OpenTarget } from '../target';
import type { NoteDoc } from '../types';
import { NoteSurface } from './note-surface';

/** The editor over the live draft (or none yet). */
function LiveEditor({ initialDoc }: { initialDoc: NoteDoc | null }) {
  const [target] = useState(liveTarget);
  const handle = useNoteEditor({ target, initialDoc, autoFocus: true });
  useNativeImageDrop(handle.editor, target);
  useMountEffect(() => {
    useLiveNotes.setState({ flush: handle.flush });
    return () => {
      useLiveNotes.setState({ flush: null });
    };
  });
  return <NoteSurface handle={handle} />;
}

/** Reopens an existing draft's notes before editing them. */
function DraftEditor({ draft }: { draft: OpenTarget }) {
  const notes = useMeetingNotes(draft.meetingId);
  if (notes.isPending) return null;
  return <LiveEditor initialDoc={notes.data?.doc ?? null} />;
}

/** One live session: the draft as it was when it mounted, then the editor owns it. */
function LiveSession() {
  const [draft] = useState(() => useLiveNotes.getState().draft);
  return draft === null ? <LiveEditor initialDoc={null} /> : <DraftEditor draft={draft} />;
}

/** The live notepad; a fresh one appears after each adopted recording. */
export function LiveNotepad() {
  const generation = useLiveNotes((state) => state.generation);
  return <LiveSession key={generation} />;
}
