/**
 * Mock of `@/lib/ipc/generated/example` (the debug-only reference feature):
 * notes in memory, `NoteAdded` on every add, and the Rust validation rules.
 */
import type * as Generated from '@/lib/ipc/generated/example';

import { latency } from './latency';
import { charCount, failure, ok } from './mock-database';
import { mockEvent } from './mock-event';

/** Same limit as `deino_core::_example::MAX_NOTE_CHARS`. */
const MAX_NOTE_CHARS = 500;

/** Saved notes, newest first. */
const notes: Generated.ExampleNote[] = [];

export const events = {
  noteAdded: mockEvent<Generated.NoteAdded>('plugin:example:note-added'),
} satisfies typeof Generated.events;

export const commands = {
  async list() {
    await latency();
    return ok([...notes]);
  },
  async add(input) {
    await latency();
    const text = input.trim();
    if (text === '') return failure({ code: 'invalid', message: 'a note cannot be empty' });
    if (charCount(text) > MAX_NOTE_CHARS) {
      const message = `a note can be at most ${MAX_NOTE_CHARS} characters long`;
      return failure({ code: 'invalid', message });
    }
    const note = { id: newNoteId(), text, createdAt: new Date().toISOString() };
    notes.unshift(note);
    void events.noteAdded.emit({ note });
    return ok(note);
  },
} satisfies typeof Generated.commands;

/** A new note ID. The mock mints it, as Rust would, so branding it is safe. */
function newNoteId(): Generated.ExampleNoteId {
  return crypto.randomUUID() as Generated.ExampleNoteId;
}
