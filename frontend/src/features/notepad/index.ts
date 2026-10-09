/**
 * The notepad ("My notes", NOT-01): a TipTap 3 editor over the `notes`
 * plugin, with Markdown shortcuts, checklists, @mentions, images, autosave
 * and focus mode.
 *
 * | Export | Where |
 * |---|---|
 * | {@link Notepad} | the meeting page's Notes tab ("My notes") |
 * | {@link LiveNotepad}, {@link adoptLiveNotes} | the record screen, and the stop path that hands its notes to the saved meeting |
 * | {@link NotepadWindow} | a stand-alone window (CAP-10's "Open notes") |
 * | {@link SummaryDocument} | the legacy AI summary, edited with the same schema |
 *
 * The editors load on first use (`lazy.tsx`). Other code imports the
 * notepad from here only.
 */
export { notesApi } from './api';
export type { NotepadProps } from './components/notepad';
export type { NotepadWindowProps } from './components/notepad-window';
export type { SummaryDocumentHandle, SummaryDocumentProps } from './components/summary-document';
export { notepadCopy } from './copy';
export { isImagePath } from './images';
export { LiveNotepad, Notepad, NotepadWindow, SummaryDocument } from './lazy';
export { adoptLiveNotes, useLiveNotes } from './live-notes';
export { meetingNotesQuery, notesKeys, useMeetingNotes } from './queries';
export { summaryMarkdown } from './summary-markdown';
export type { MeetingNotes, NoteDoc, SaveStatus } from './types';
