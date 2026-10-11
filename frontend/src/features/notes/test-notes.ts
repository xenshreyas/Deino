/**
 * Enhanced notes for tests: a view with an AI line that cites the
 * transcript, a user line and three sections (so the contents show).
 */
import { asMeetingId } from '@/features/meetings';
import { type NoteDoc, toNoteDoc } from '@/features/notepad';

import type { EnhancedNotesView, VersionInfo } from './types';

export const TEST_MEETING = asMeetingId('meeting-fixture-one-on-one');

/** The cited segment of {@link testDoc}'s AI line. */
export const CITED_SEGMENT = '0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e';

/** A text run by `author`. */
function run(text: string, author: 'ai' | 'user') {
  return { type: 'text', text, marks: [{ type: 'origin', attrs: { author, edited: false } }] };
}

/** A heading written by the AI. */
function heading(text: string) {
  return { type: 'heading', attrs: { level: 3 }, content: [run(text, 'ai')] };
}

/** A list of one bullet. */
function bullets(content: unknown[]) {
  return {
    type: 'bulletList',
    content: [{ type: 'listItem', content: [{ type: 'paragraph', content }] }],
  };
}

/** Overview (one cited AI line), Your notes (one user line), Next steps. */
export function testDoc(): NoteDoc {
  return toNoteDoc({
    type: 'doc',
    content: [
      heading('Overview'),
      bullets([
        run('Maya comes off the on-call rotation. ', 'ai'),
        { type: 'citation', attrs: { segmentId: CITED_SEGMENT, startMs: 37_000 } },
      ]),
      heading('Your notes'),
      bullets([run('send her PM draft thurs', 'user')]),
      heading('Next steps'),
    ],
  });
}

/** Version `version`'s entry in the history. */
export function testVersion(version: number, isUserEdited = false): VersionInfo {
  return {
    version,
    createdAt: '2026-10-07T16:21:35Z',
    updatedAt: '2026-10-07T16:21:35Z',
    templateId: null,
    templateName: '1:1',
    model: 'anthropic:claude-sonnet-5-5',
    isUserEdited,
  };
}

/** The newest version of the test notes. */
export function testView(overrides: Partial<EnhancedNotesView> = {}): EnhancedNotesView {
  return {
    meetingId: TEST_MEETING,
    version: testVersion(1),
    isLatest: true,
    doc: testDoc(),
    reasons: [],
    ...overrides,
  };
}
