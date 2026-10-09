/**
 * The record screen's notes, written before a meeting row exists.
 *
 * Until FND-05b creates the meeting when recording starts, the record
 * screen's notepad creates a `scheduled` draft meeting on its first edit and
 * autosaves into it, so every keystroke is on disk within a second. When the
 * legacy stop path has saved the recording as its own meeting,
 * {@link adoptLiveNotes} flushes the last edits and moves the notes (and
 * images) into it; the draft is deleted. Without a recording, the draft
 * simply stays a scheduled note in the meetings list.
 */
import { toast } from 'sonner';
import { create } from 'zustand';

import type { MeetingId } from '@/features/meetings';

import { notesApi } from './api';
import { notepadCopy } from './copy';
import type { NoteTarget, OpenTarget } from './target';

/** The live draft, shared by the record screen and the stop path. */
interface LiveNotesState {
  draft: OpenTarget | null;
  creating: Promise<OpenTarget> | null;
  /** Bumped when a draft is adopted, so the notepad remounts empty. */
  generation: number;
  /** Saves the live editor's pending edits; set while it is mounted. */
  flush: (() => Promise<void>) | null;
}

/** The live notes store; select single fields. */
export const useLiveNotes = create<LiveNotesState>()(() => ({
  draft: null,
  creating: null,
  generation: 0,
  flush: null,
}));

async function createDraft(title: string): Promise<OpenTarget> {
  const meetingId = await notesApi.createScheduled(title);
  const { folder } = await notesApi.get(meetingId);
  return { meetingId, folder };
}

/** The draft, created once even if two saves race for it. */
async function ensureDraft(title: string): Promise<OpenTarget> {
  const { draft, creating } = useLiveNotes.getState();
  if (draft !== null) return draft;
  if (creating !== null) return creating;
  const pending = createDraft(title);
  useLiveNotes.setState({ creating: pending });
  try {
    const created = await pending;
    useLiveNotes.setState({ draft: created, creating: null });
    return created;
  } catch (error) {
    useLiveNotes.setState({ creating: null });
    throw error;
  }
}

/** A draft's title, "New note · 9 Oct, 14:05"; a recording's own title replaces it. */
function draftTitle(now: Date): string {
  const when = now.toLocaleString('en-GB', {
    day: 'numeric',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
  });
  return `${notepadCopy.draftTitle} · ${when}`;
}

/** The record screen's target; the first edit creates the draft. */
export function liveTarget(): NoteTarget {
  return {
    resolve: () => ensureDraft(draftTitle(new Date())),
    folder: () => useLiveNotes.getState().draft?.folder ?? null,
  };
}

/**
 * Moves the live notes into the meeting a recording was saved as. Never
 * throws: if the move fails, the notes stay in their draft, which is a
 * meeting of its own, and the user is told where they are.
 */
export async function adoptLiveNotes(into: MeetingId): Promise<void> {
  const { flush, creating } = useLiveNotes.getState();
  await flush?.();
  await creating?.catch(() => undefined);
  const { draft } = useLiveNotes.getState();
  if (draft === null) return;
  try {
    await notesApi.adopt(draft.meetingId, into);
    useLiveNotes.setState((state) => ({ draft: null, generation: state.generation + 1 }));
  } catch {
    toast.error(notepadCopy.adoptFailedTitle, { description: notepadCopy.adoptFailedBody });
  }
}
