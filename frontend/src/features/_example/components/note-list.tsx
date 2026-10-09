import { cn } from '@/lib/utils';

import { copy } from '../copy';
import type { ExampleNote, ExampleNoteId } from '../types';

interface NoteListProps {
  notes: readonly ExampleNote[];
  /** The note to highlight because it just arrived, if any. */
  latestId: ExampleNoteId | null;
}

/** The saved notes, newest first, with an empty state. Presentational. */
export function NoteList({ notes, latestId }: NoteListProps) {
  if (notes.length === 0) {
    return <p className="text-sm text-muted-foreground">{copy.empty}</p>;
  }
  return (
    <ul className="flex flex-col gap-1">
      {notes.map((note) => (
        <li
          key={note.id}
          className={cn(
            'rounded-md px-3 py-2 text-sm transition-colors duration-150',
            note.id === latestId && 'bg-accent text-accent-foreground',
          )}
        >
          {note.text}
        </li>
      ))}
    </ul>
  );
}
