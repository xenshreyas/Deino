/**
 * Typed calls to the `notes` and `people` plugins: the notepad's only IPC.
 * Each resolves to the data or throws an `IpcError`.
 */
import { convertFileSrc } from '@tauri-apps/api/core';

import type { MeetingId } from '@/features/meetings';
import { commands as notes } from '@/lib/ipc/generated/notes';
import { commands as people } from '@/lib/ipc/generated/people';
import { unwrap } from '@/lib/ipc/result';

import type { MeetingNotes, NoteDoc, NoteImage, Person, SavedNotes } from './types';

export const notesApi = {
  /** The meeting's notes; an empty document before the first save. */
  get: (meetingId: MeetingId): Promise<MeetingNotes> => unwrap(notes.get(meetingId)),
  /** Saves the whole document (the autosave). */
  save: (meetingId: MeetingId, doc: NoteDoc): Promise<SavedNotes> =>
    unwrap(notes.save(meetingId, doc)),
  /** Stores a pasted image (base64, without a `data:` prefix). */
  saveImage: (meetingId: MeetingId, base64: string): Promise<NoteImage> =>
    unwrap(notes.saveImage(meetingId, base64)),
  /** Stores a file dropped on the window, by the path the drop reported. */
  attachDroppedImage: (meetingId: MeetingId, path: string): Promise<NoteImage> =>
    unwrap(notes.attachDroppedImage(meetingId, path)),
  /** Markdown → document, through `deino-doc`. */
  markdownToDoc: (markdown: string): Promise<NoteDoc> => unwrap(notes.markdownToDoc(markdown)),
  /** Document → Lossless Markdown, through `deino-doc`. */
  docToMarkdown: (doc: NoteDoc): Promise<string> => unwrap(notes.docToMarkdown(doc)),
};

export const peopleApi = {
  /** People whose name, a word of it, or email starts with `query`. */
  search: (query: string): Promise<Person[]> => unwrap(people.search(query)),
};

/**
 * The URL an image node shows: its source (`images/<id>.png`) inside the
 * meeting's folder, through Tauri's asset protocol.
 */
export function imageUrl(folder: string, src: string): string {
  return convertFileSrc(`${folder}/${src}`);
}
