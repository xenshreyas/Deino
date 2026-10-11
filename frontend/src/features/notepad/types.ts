/**
 * The notepad's types. The document model is generated from `deino-doc`
 * (specta names the TipTap-tagged structs `doc`, `listItem` and `taskItem`;
 * they are re-exported here under type-style names).
 */
export type {
  Block,
  Inline,
  Mark,
  MeetingNotes,
  NoteImage,
  doc as NoteDoc,
  SavedNotes,
} from '@/lib/ipc/generated/notes';
export type { Person, PersonId } from '@/lib/ipc/generated/people';

/** Where an autosaving document stands, for the quiet "Saved" indicator. */
export type SaveStatus = 'idle' | 'pending' | 'saving' | 'saved' | 'error';

/** A person offered by the @mention menu. */
export interface MentionCandidate {
  id: string;
  label: string;
  /** A second line: their email, when known. */
  detail: string | null;
  /** Set on the "Add “Name”" entry: choosing it adds someone new (ORG-04). */
  isNew?: true;
}
