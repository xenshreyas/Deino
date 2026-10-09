import { ICON_PROPS, NotebookPen } from '@/ui/icons';

import { copy } from '../copy';
import { useLatestNote } from '../hooks/use-latest-note';
import { useAddExampleNote, useExampleNotes } from '../queries';
import { AddNoteForm } from './add-note-form';
import { NoteList } from './note-list';

/**
 * The example feature's screen: the container that connects data to the
 * presentational `AddNoteForm` and `NoteList`. It holds no state of its own:
 * the notes come from a query, saving is a mutation, and the highlight comes
 * from Rust's typed `NoteAdded` event.
 */
export function ExampleNotes() {
  const notes = useExampleNotes();
  const addNote = useAddExampleNote();
  const latestId = useLatestNote();

  return (
    <section className="mx-auto flex max-w-[720px] flex-col gap-4 p-6">
      <h1 className="flex items-center gap-2 text-2xl font-semibold">
        <NotebookPen {...ICON_PROPS} />
        {copy.title}
      </h1>
      <AddNoteForm
        saving={addNote.isPending}
        error={addNote.error?.message}
        onAdd={(text, onSaved) => {
          addNote.mutate(text, { onSuccess: onSaved });
        }}
      />
      {notes.isPending ? (
        <p className="text-sm text-fg-muted">{copy.loading}</p>
      ) : (
        <NoteList notes={notes.data ?? []} latestId={latestId} />
      )}
    </section>
  );
}
