/**
 * The Meetings page in the mock (ORG-07): filtered cursor pages over the
 * fixture meetings, saved views, the trash and merge, with the Rust rules
 * that matter on screen. Filters by date, people, companies, tags and
 * folders; "has action items" and internal/external pass everything (the
 * Rust contract tests cover them).
 */
import type * as Generated from '@/lib/ipc/generated/meetings';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import { mockDatabase, mockEntities } from './database';
import { mockFolders, mockTags } from './organizer';

type Meeting = Generated.Meeting;
type MeetingId = Generated.MeetingId;
type Filter = Generated.MeetingFilter;

/** How long a meeting stays in the trash, as in `TRASH_RETENTION`. */
const RETENTION_DAYS = 30;

/** True when `meeting` passes `filter`'s supported conditions. */
function matches(meeting: Meeting, filter: Filter): boolean {
  const day = meeting.createdAt.slice(0, 10);
  const range = filter.dateRange;
  if (range?.from != null && day < range.from) return false;
  if (range?.to != null && day > range.to) return false;
  const people = mockEntities.attendance.filter((row) => row.meetingId === meeting.id);
  const anyOf = <T>(wanted: readonly T[], has: (value: T) => boolean) =>
    wanted.length === 0 || wanted.some(has);
  const tags = mockTags.tagsOf(meeting.id).map((applied) => applied.tag.id);
  const folders = mockFolders.foldersOf(meeting.id).map((folder) => folder.id);
  return (
    anyOf(filter.people, (id) => people.some((row) => row.personId === id)) &&
    anyOf(filter.companies, (id) =>
      people.some((row) => mockEntities.person(row.personId)?.companyId === id),
    ) &&
    anyOf(filter.tags, (id) => tags.includes(id)) &&
    anyOf(filter.folders, (id) => folders.includes(id))
  );
}

/** A list row for `meeting`. */
function listed(meeting: Meeting): Generated.ListedMeeting {
  const durationMs = mockDatabase.fixture(meeting.id)?.durationMs;
  const attendees = mockEntities.attendance
    .filter((row) => row.meetingId === meeting.id)
    .flatMap((row) => {
      const person = mockEntities.person(row.personId);
      return person === undefined
        ? []
        : [
            {
              personId: person.id,
              name: person.name,
              avatarPath: person.avatarPath,
              isMe: person.isMe,
            },
          ];
    });
  return {
    id: meeting.id,
    title: meeting.title,
    createdAt: meeting.createdAt,
    startedAt: meeting.createdAt,
    duration: durationMs === undefined ? null : Math.round(durationMs / 1000),
    status: 'ready',
    attendees,
    tags: mockTags
      .tagsOf(meeting.id)
      .map(({ tag }) => ({ id: tag.id, name: tag.name, color: tag.color })),
  };
}

/** The list, the views and the trash. */
export class MockMeetingsList {
  private trashed = new Map<MeetingId, string>();
  private gone = new Set<MeetingId>();
  private savedViews: Generated.MeetingView[] = [];

  /** Meetings outside the trash that `filter` matches, newest first. */
  matching(filter: Filter): Meeting[] {
    return mockDatabase
      .meetings()
      .filter((m) => !this.trashed.has(m.id) && !this.gone.has(m.id) && matches(m, filter));
  }

  page(filter: Filter, after: Generated.MeetingCursor | null, size: number): Generated.MeetingPage {
    const all = this.matching(filter);
    const start = after === null ? 0 : all.findIndex((m) => m.id === after.id) + 1;
    const rows = all.slice(start, start + Math.max(1, size));
    const last = rows.at(-1);
    const more = start + rows.length < all.length && last !== undefined;
    return { meetings: rows.map(listed), next: more ? { at: last.createdAt, id: last.id } : null };
  }

  views(): Generated.MeetingView[] {
    return this.savedViews.map((view) => ({ ...view }));
  }

  saveView(
    id: Generated.MeetingViewId | null,
    input: Generated.MeetingViewInput,
  ): CommandResult<Generated.MeetingView> {
    const name = input.name.trim();
    const clash = this.savedViews.some(
      (v) => v.id !== id && v.name.toLowerCase() === name.toLowerCase(),
    );
    if (name === '') return failure({ code: 'invalid', message: 'a view name cannot be empty' });
    if (clash)
      return failure({ code: 'conflict', message: 'a view with this name already exists' });
    const now = new Date().toISOString();
    const existing = this.savedViews.find((view) => view.id === id);
    const view: Generated.MeetingView = existing
      ? { ...existing, name, filter: input.filter, updatedAt: now }
      : {
          id: crypto.randomUUID() as Generated.MeetingViewId,
          name,
          filter: input.filter,
          position: this.savedViews.length,
          createdAt: now,
          updatedAt: now,
        };
    this.savedViews = existing
      ? this.savedViews.map((v) => (v.id === view.id ? view : v))
      : [...this.savedViews, view];
    mockDatabase.touch(['meeting_views']);
    return ok({ ...view });
  }

  deleteView(id: Generated.MeetingViewId): CommandResult<null> {
    if (!this.savedViews.some((view) => view.id === id))
      return failure({ code: 'not_found', message: `view ${id} was not found` });
    this.savedViews = this.savedViews.filter((view) => view.id !== id);
    mockDatabase.touch(['meeting_views']);
    return ok(null);
  }

  trash(ids: readonly MeetingId[]): Generated.TrashOutcome {
    const live = new Set(this.matching(ALL).map((m) => m.id));
    const changed = [...new Set(ids)].filter((id) => live.has(id));
    for (const id of changed) this.trashed.set(id, new Date().toISOString());
    mockDatabase.touch(['meetings']);
    return { changed };
  }

  restore(ids: readonly MeetingId[]): Generated.TrashOutcome {
    const changed = [...new Set(ids)].filter((id) => this.trashed.delete(id));
    mockDatabase.touch(['meetings']);
    return { changed };
  }

  trashedMeetings(): Generated.TrashedMeeting[] {
    const byId = new Map(mockDatabase.meetings().map((m) => [m.id, m]));
    return [...this.trashed.entries()]
      .sort(([, a], [, b]) => b.localeCompare(a))
      .flatMap(([id, deletedAt]) => {
        const meeting = byId.get(id);
        if (meeting === undefined) return [];
        const purgesAt = new Date(
          Date.parse(deletedAt) + RETENTION_DAYS * 86_400_000,
        ).toISOString();
        return [{ id, title: meeting.title, createdAt: meeting.createdAt, deletedAt, purgesAt }];
      });
  }

  deleteForever(ids: readonly MeetingId[]): Generated.TrashOutcome {
    const changed = [...new Set(ids)].filter((id) => this.trashed.delete(id));
    for (const id of changed) this.gone.add(id);
    mockDatabase.touch(['meetings']);
    return { changed };
  }

  merge(ids: readonly MeetingId[]): CommandResult<Generated.MergeOutcome> {
    const chosen = this.matching(ALL)
      .filter((m) => ids.includes(m.id))
      .reverse();
    const [first, ...rest] = chosen;
    if (first === undefined || rest.length === 0)
      return failure({ code: 'invalid', message: 'choose at least two meetings to merge' });
    for (const meeting of rest) this.gone.add(meeting.id);
    mockDatabase.touch(['meetings', 'transcript_segments', 'meeting_merge_parts']);
    return ok({ meetingId: first.id, absorbed: rest.map((m) => m.id) });
  }
}

/** The filter of "All meetings". */
const ALL: Filter = {
  dateRange: null,
  people: [],
  companies: [],
  tags: [],
  folders: [],
  hasActionItems: false,
  audience: null,
};

/** The mock's Meetings page state. */
export const mockMeetingsList = new MockMeetingsList();
