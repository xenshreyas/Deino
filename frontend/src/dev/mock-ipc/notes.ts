/**
 * Mock of `@/lib/ipc/generated/notes`: the notepad over the mock notes table
 * (persisted per tab, so a reload keeps typed notes). Markdown conversion
 * uses the small stand-in converter; the real app converts in Rust.
 */
import type * as Generated from '@/lib/ipc/generated/notes';

import { mockDatabase, mockNotes } from './database';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { mockEvent } from './mock-event';
import { docToMarkdown, markdownToDoc } from './mock-markdown';

export const events = {
  notesSaved: mockEvent<Generated.NotesSaved>('plugin:notes:notes-saved'),
} satisfies typeof Generated.events;

function notFound(meetingId: string) {
  return failure<never>({ code: 'not_found', message: `meeting ${meetingId} was not found` });
}

export const commands = {
  async get(meetingId) {
    await latency();
    if (mockDatabase.meeting(meetingId).status === 'error') return notFound(meetingId);
    return ok(mockNotes.get(meetingId));
  },
  async save(meetingId, doc) {
    await latency();
    if (mockDatabase.meeting(meetingId).status === 'error') return notFound(meetingId);
    const saved = mockNotes.save(meetingId, doc, new Date());
    mockDatabase.touch(['meeting_notes']);
    void events.notesSaved.emit({ saved });
    return ok(saved);
  },
  async createScheduled(title) {
    await latency();
    return mockDatabase.addScheduled(title, new Date());
  },
  async saveImage(meetingId, base64) {
    await latency();
    return ok({ src: mockNotes.saveImage(meetingId, base64) });
  },
  async attachDroppedImage() {
    await latency();
    return failure({ code: 'invalid', message: 'that file was not dropped on the window' });
  },
  async markdownToDoc(markdown) {
    await latency();
    return ok(markdownToDoc(markdown));
  },
  async docToMarkdown(doc) {
    await latency();
    return ok(docToMarkdown(doc));
  },
} satisfies typeof Generated.commands;
