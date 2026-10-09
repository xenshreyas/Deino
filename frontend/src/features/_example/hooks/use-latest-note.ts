import { useState } from 'react';

import { events } from '@/lib/ipc/generated/example';
import { useIpcEvent } from '@/lib/ipc/events';

import type { ExampleNoteId } from '../types';

/**
 * The ID of the note most recently announced by Rust's typed `NoteAdded`
 * event, or `null` before the first one. Components use it to highlight the
 * note that just arrived.
 */
export function useLatestNote(): ExampleNoteId | null {
  const [latest, setLatest] = useState<ExampleNoteId | null>(null);
  useIpcEvent(events.noteAdded, ({ note }) => {
    setLatest(note.id);
  });
  return latest;
}
