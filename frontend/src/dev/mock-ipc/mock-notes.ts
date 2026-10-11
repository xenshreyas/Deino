/**
 * The mock `meeting_notes` table: the fixtures' `userNotes` (converted like
 * the Rust seeder does), plus whatever the notepad saves. It persists in
 * `sessionStorage`, so a reload keeps typed notes (the Playwright notepad
 * test relies on it) while a new tab starts from the fixtures. Pasted
 * images are kept as data URLs and served through the mocked asset protocol.
 */
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { doc as NoteDoc, MeetingNotes, SavedNotes } from '@/lib/ipc/generated/notes';

import type { FixtureMeeting } from './fixture-schema';
import { markdownToDoc } from './mock-markdown';

/** The `sessionStorage` key of the saved state. */
const STORAGE_KEY = 'deino.mock.notes';

/** The folder prefix the mock hands out; the asset mock resolves it. */
export const MOCK_FOLDER = 'mock-meetings';

interface StoredNote {
  doc: NoteDoc;
  updatedAt: string;
}

interface State {
  notes: Record<string, StoredNote>;
  images: Record<string, string>;
}

function fromFixtures(fixtures: readonly FixtureMeeting[]): State {
  const notes: Record<string, StoredNote> = {};
  for (const fixture of fixtures) {
    if (fixture.userNotes === null) continue;
    const savedAt = new Date(Date.parse(fixture.startedAt) + fixture.durationMs).toISOString();
    notes[fixture.id] = { doc: markdownToDoc(fixture.userNotes), updatedAt: savedAt };
  }
  return { notes, images: {} };
}

function restore(): State | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    return raw === null ? null : (JSON.parse(raw) as State);
  } catch {
    return null;
  }
}

/** The notes behind the `notes` mock. */
export class MockNotes {
  private state: State;

  constructor(private readonly fixtures: readonly FixtureMeeting[]) {
    this.state = restore() ?? fromFixtures(fixtures);
  }

  /** A meeting's notes, or an empty document. */
  get(meetingId: MeetingId): MeetingNotes {
    const stored = this.state.notes[meetingId];
    return {
      meetingId,
      doc: stored?.doc ?? { type: 'doc', content: [{ type: 'paragraph', content: [] }] },
      updatedAt: stored?.updatedAt ?? null,
      folder: `${MOCK_FOLDER}/${meetingId}`,
    };
  }

  /** Saves a document, like the autosave. */
  save(meetingId: MeetingId, doc: NoteDoc, now: Date): SavedNotes {
    const updatedAt = now.toISOString();
    this.state.notes[meetingId] = { doc, updatedAt };
    this.persist();
    return { meetingId, updatedAt };
  }

  /** Keeps an image and returns its source. */
  saveImage(meetingId: MeetingId, base64: string): string {
    const src = `images/${crypto.randomUUID()}.png`;
    this.state.images[`${MOCK_FOLDER}/${meetingId}/${src}`] = `data:image/png;base64,${base64}`;
    this.persist();
    return src;
  }

  /** The data URL of an image path the asset protocol was asked for. */
  imageUrl(path: string): string | undefined {
    return this.state.images[path];
  }

  /** Back to the fixtures, like a reseed. */
  reseed(): void {
    this.state = fromFixtures(this.fixtures);
    this.persist();
  }

  private persist(): void {
    try {
      sessionStorage.setItem(STORAGE_KEY, JSON.stringify(this.state));
    } catch {
      // A full or blocked storage only costs persistence across reloads.
    }
  }
}
