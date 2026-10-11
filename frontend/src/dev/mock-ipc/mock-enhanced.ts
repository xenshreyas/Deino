/**
 * The mock `enhanced_notes` table: version 1 for every fixture meeting
 * except the standup and the support call (which open on "My notes" and
 * show Enhance), plus whatever the UI writes, edits or restores. It
 * persists in `sessionStorage`, like `MockNotes`, so a reload keeps edits.
 */
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type {
  doc as NoteDoc,
  EnhancedNotesView,
  RegenerateReason,
  VersionInfo,
} from '@/lib/ipc/generated/enhance';

import type { FixtureMeeting } from './fixture-schema';
import { mockEnhancedDoc } from './mock-enhanced-doc';

/** The `sessionStorage` key of the saved state. */
const STORAGE_KEY = 'deino.mock.enhanced';
/** Fixtures written without notes, to show the empty Enhanced view. */
const WITHOUT_NOTES = new Set(['meeting-fixture-standup', 'meeting-fixture-customer-support']);
/** The model the mock "writes" with. */
const MODEL = 'anthropic:claude-sonnet-5-5';
/** Each fixture's template, as auto selection picks it (NOT-04). */
const TEMPLATES: Readonly<Record<FixtureMeeting['kind'], [string, string]>> = {
  one_on_one: ['one_on_one', '1:1'],
  standup: ['standup', 'Stand-up'],
  sales_discovery: ['sales_discovery', 'Sales discovery'],
  interview: ['candidate_interview', 'Candidate interview'],
  board_update: ['board_meeting', 'Board meeting'],
  customer_support: ['customer_support', 'Customer support call'],
  brainstorm: ['brainstorm', 'Brainstorm'],
  planning: ['general', 'General'],
};

/** How a version came to be. */
interface Written {
  at: Date;
  fixture: FixtureMeeting | undefined;
  edited?: boolean;
}

interface StoredVersion {
  info: VersionInfo;
  doc: NoteDoc;
}

type State = Record<string, StoredVersion[]>;

function restore(): State | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    return raw === null ? null : (JSON.parse(raw) as State);
  } catch {
    return null;
  }
}

/** The enhanced notes behind the `enhance` mock. */
export class MockEnhanced {
  private state: State;

  constructor(private readonly fixtures: readonly FixtureMeeting[]) {
    this.state = restore() ?? this.seeded();
  }

  /** A version (the newest for `null`) as the view shows it; `undefined` when missing. */
  view(
    meetingId: MeetingId,
    version: number | null,
    notesSavedAt: string | null,
  ): EnhancedNotesView | null | undefined {
    const versions = this.state[meetingId] ?? [];
    const newest = versions.at(-1);
    if (newest === undefined) return version === null ? null : undefined;
    const shown = version === null ? newest : versions.find((v) => v.info.version === version);
    if (shown === undefined) return undefined;
    const isLatest = shown === newest;
    const reasons: RegenerateReason[] =
      isLatest && notesSavedAt !== null && notesSavedAt > shown.info.createdAt
        ? ['notesEdited']
        : [];
    return { meetingId, version: { ...shown.info }, isLatest, doc: shown.doc, reasons };
  }

  /** Every version, newest first. */
  versions(meetingId: MeetingId): VersionInfo[] {
    return (this.state[meetingId] ?? []).map((v) => ({ ...v.info })).reverse();
  }

  /** True when the newest version carries the user's edits. */
  isEdited(meetingId: MeetingId): boolean {
    return this.state[meetingId]?.at(-1)?.info.isUserEdited ?? false;
  }

  /** Writes a new version from the fixture; its number. */
  write(meetingId: MeetingId, now: Date): number {
    const fixture = this.fixtures.find((candidate) => candidate.id === meetingId);
    const doc = fixture === undefined ? emptyDoc() : mockEnhancedDoc(fixture);
    return this.push(meetingId, doc, { at: now, fixture });
  }

  /** Saves an edit of the newest version; false for an older one. */
  save(meetingId: MeetingId, version: number, doc: NoteDoc, now: Date): boolean {
    const newest = this.state[meetingId]?.at(-1);
    if (newest?.info.version !== version) return false;
    newest.doc = doc;
    newest.info = { ...newest.info, isUserEdited: true, updatedAt: now.toISOString() };
    this.persist();
    return true;
  }

  /** Copies an older version as the newest; its number, or `null` when missing. */
  restoreVersion(meetingId: MeetingId, version: number, now: Date): number | null {
    const source = this.state[meetingId]?.find((v) => v.info.version === version);
    if (source === undefined) return null;
    const fixture = this.fixtures.find((candidate) => candidate.id === meetingId);
    return this.push(meetingId, source.doc, { at: now, fixture, edited: source.info.isUserEdited });
  }

  /** Back to the fixtures, like a reseed. */
  reseed(): void {
    this.state = this.seeded();
    this.persist();
  }

  private push(
    meetingId: MeetingId,
    doc: NoteDoc,
    { at: now, fixture, edited = false }: Written,
  ): number {
    const versions = this.state[meetingId] ?? [];
    const version = (versions.at(-1)?.info.version ?? 0) + 1;
    const at = now.toISOString();
    const [templateId, templateName] =
      fixture === undefined ? [null, null] : TEMPLATES[fixture.kind];
    const info: VersionInfo = {
      version,
      createdAt: at,
      updatedAt: at,
      templateId: templateId as VersionInfo['templateId'],
      templateName,
      model: MODEL,
      isUserEdited: edited,
    };
    this.state[meetingId] = [...versions, { info, doc }];
    this.persist();
    return version;
  }

  private seeded(): State {
    this.state = {};
    for (const fixture of this.fixtures) {
      if (WITHOUT_NOTES.has(fixture.id)) continue;
      const writtenAt = new Date(Date.parse(fixture.startedAt) + fixture.durationMs + 60_000);
      this.push(fixture.id as MeetingId, mockEnhancedDoc(fixture), { at: writtenAt, fixture });
    }
    return this.state;
  }

  private persist(): void {
    try {
      sessionStorage.setItem(STORAGE_KEY, JSON.stringify(this.state));
    } catch {
      // A full or blocked storage only costs persistence across reloads.
    }
  }
}

function emptyDoc(): NoteDoc {
  return { type: 'doc', content: [{ type: 'paragraph', content: [] }] };
}
