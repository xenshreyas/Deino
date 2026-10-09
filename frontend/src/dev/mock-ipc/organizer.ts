/**
 * The folders and tags `pnpm dev:mock` starts with, over the fixture
 * meetings: a few tags (some applied by hand, some by the auto-tagger) and
 * a small folder tree, with stable `fixtureUuid` IDs. The Rust seeder seeds
 * no folders or tags (the fixtures carry none); these exist so the screens
 * have something to show (PARITY_PLAN §7.2, FND-07 follow-up).
 */
import type * as Folders from '@/lib/ipc/generated/folders';
import type * as Tags from '@/lib/ipc/generated/tags';

import { mockDatabase } from './database';
import { events } from './db';
import { fixtureUuid } from './fixture-ids';
import { MockFolders } from './mock-folders';
import { MockTags } from './mock-tags';

/** When the seeded rows were made. */
const SEEDED_AT = '2026-10-01T09:00:00.000Z';

let seq = 1_000_000;
function emitChange(tables: Parameters<typeof events.dbChanged.emit>[0]['tables']) {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
}

function meeting(slug: string): Folders.MeetingId {
  return `meeting-fixture-${slug}` as Folders.MeetingId;
}

function tag(key: string, name: string, color: number, description: string): Tags.Tag {
  const id = fixtureUuid(`tag/${key}`) as Tags.TagId;
  return {
    id,
    name,
    color,
    description,
    autoApply: true,
    createdAt: SEEDED_AT,
    updatedAt: SEEDED_AT,
  };
}

const TAGS = [
  tag('customer', 'Customer', 1, 'Calls with paying customers or prospects about their account.'),
  tag('hiring', 'Hiring', 2, 'Interviews, debriefs and anything about filling a role.'),
  tag('leadership', 'Leadership', 5, 'Board meetings, investor updates and company strategy.'),
  tag(
    'calendar-sync',
    'Calendar Sync',
    3,
    'Work on calendar sync: v1.4 incident, v2 scope, recurring events.',
  ),
];

function link(slug: string, tagIndex: number, appliedBy: Tags.TagSource): Tags.MeetingTagLink {
  const tagId = TAGS[tagIndex]?.id ?? ('' as Tags.TagId);
  return { meetingId: meeting(slug) as Tags.MeetingId, tagId, appliedBy };
}

/** A seeded folder: its fixture key, name, parent key, icon and description. */
interface FolderSeed {
  key: string;
  name: string;
  parent: string | null;
  icon: string | null;
  description?: string;
}

function folderId(key: string): Folders.FolderId {
  return fixtureUuid(`folder/${key}`) as Folders.FolderId;
}

function folder({ key, name, parent, icon, description = '' }: FolderSeed): Folders.Folder {
  const parentId = parent === null ? null : folderId(parent);
  const stamps = { createdAt: SEEDED_AT, updatedAt: SEEDED_AT };
  return { id: folderId(key), parentId, name, icon, description, ...stamps };
}

const FOLDERS = [
  {
    key: 'company',
    name: 'Company',
    parent: null,
    icon: 'building',
    description: 'Leadership, board and planning meetings.',
  },
  { key: 'board', name: 'Board', parent: 'company', icon: 'flag' },
  {
    key: 'customers',
    name: 'Customers',
    parent: null,
    icon: 'users',
    description: 'Every conversation with a customer or prospect.',
  },
].map(folder);

/** The mock's tags. */
export const mockTags = new MockTags(
  TAGS,
  [
    link('sales-discovery', 0, 'user'),
    link('customer-support', 0, 'ai'),
    link('customer-support', 3, 'ai'),
    link('interview', 1, 'user'),
    link('board-update', 2, 'ai'),
    link('planning', 3, 'user'),
  ],
  emitChange,
);

/** The mock's folders. */
export const mockFolders = new MockFolders(
  {
    folders: FOLDERS,
    filings: [
      { folderId: folderId('board'), meetingId: meeting('board-update') },
      { folderId: folderId('company'), meetingId: meeting('planning') },
      { folderId: folderId('customers'), meetingId: meeting('sales-discovery') },
    ],
    rules: [],
  },
  {
    meetings: () => mockDatabase.meetings(),
    seriesOf: (id) => {
      const event = mockDatabase.fixture(id)?.calendarEvent;
      return event === undefined || event.recurrence === 'none'
        ? null
        : (event.icalUid as Folders.SeriesId);
    },
    onChange: emitChange,
  },
);
