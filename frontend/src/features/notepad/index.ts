/**
 * The notepad ("My notes", NOT-01): a TipTap 3 editor over the `notes`
 * plugin, with Markdown shortcuts, checklists, @mentions, images, autosave
 * and focus mode.
 *
 * | Export | Where |
 * |---|---|
 * | {@link Notepad} | the meeting page's Notes tab ("My notes"), and the record screen, which writes into the meeting being recorded |
 * | {@link NotepadWindow} | a stand-alone window (CAP-10's "Open notes") |
 * | {@link loadNoteExtensions}, {@link Autosaver}, {@link SavedIndicator}, {@link toNoteDoc} | the enhanced notes (`features/notes`, NOT-02), which edit with the same schema |
 *
 * The editors load on first use (`lazy.tsx`). Other code imports the
 * notepad from here only.
 */
export { notesApi } from './api';
export { Autosaver } from './autosave';
export { SavedIndicator } from './components/saved-indicator';
export { toNoteDoc } from './doc-schema';
export type { NoteExtensionOptions } from './editor/extensions';
export type { NotepadProps } from './components/notepad';
export type { NotepadWindowProps } from './components/notepad-window';
export { notepadCopy } from './copy';
export { isImagePath } from './images';
export { Notepad, NotepadWindow } from './lazy';
export { meetingNotesQuery, notesKeys, useMeetingNotes } from './queries';
export type { MeetingNotes, NoteDoc, SaveStatus } from './types';

/**
 * The note schema's TipTap extensions, loaded on demand: NOT-02's enhanced
 * notes edit with the same schema, and TipTap stays out of the entry chunk.
 */
export const loadNoteExtensions = () =>
  import('./editor/extensions').then((module) => module.noteExtensions);
