/**
 * Mock of `@/lib/ipc/generated/notes-edit`: proposes edits of the mock
 * enhanced notes (`mock-note-edit.ts`: a plain rename without "AI", or a
 * canned AI bullet after a pause), refuses a stale version with `conflict`
 * like Rust, and stores an accepted edit as a new edited version.
 */
import type * as Generated from '@/lib/ipc/generated/notes-edit';

import { mockDatabase, mockEnhanced } from './database';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { mockAddition, mockRename } from './mock-note-edit';

/** How long the mock "AI" takes to draft an edit. */
const DRAFTING_MS = 1200;

const STALE = 'The notes changed since you asked. Ask again on the newest version.';

export const commands = {
  async propose(request) {
    await latency();
    const view = mockEnhanced.view(request.meetingId, null, null);
    if (view === null || view === undefined) {
      return failure({ code: 'not_found', message: 'There are no enhanced notes to edit yet.' });
    }
    const base = view.version.version;
    if (base !== request.version) return failure({ code: 'conflict', message: STALE });
    const renamed = mockRename(view.doc, request, base);
    if (renamed !== null) return ok(renamed);
    await new Promise((resolve) => setTimeout(resolve, DRAFTING_MS));
    return ok(mockAddition(view.doc, request, base));
  },
  async accept(meetingId, baseVersion, doc) {
    await latency();
    const version = mockEnhanced.writeEdit(meetingId, baseVersion, doc, new Date());
    if (version === null) return failure({ code: 'conflict', message: STALE });
    mockDatabase.touch(['enhanced_notes']);
    return ok({ version, previousVersion: baseVersion });
  },
} satisfies typeof Generated.commands;
