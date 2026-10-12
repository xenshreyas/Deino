/**
 * Mock of `@/lib/ipc/generated/action-items`: the fixtures' ground-truth
 * action items (`mock-action-items.ts`), filtered with the Rust rules
 * (`mock-action-item-filter.ts`), editable for the session. Writes report
 * `action_items` or `action_item_views` through the mock database, so
 * `DbChanged` refreshes every query as in the app.
 */
import type * as Generated from '@/lib/ipc/generated/action-items';

import { fixtureMeetings, mockDatabase, mockPeople } from './database';
import { latency } from './latency';
import { byDue, matches } from './mock-action-item-filter';
import {
  edited,
  fixtureActionItems,
  fixtureCompanies,
  localNoon,
  personOptions,
  rowCompanies,
} from './mock-action-items';
import { failure, ok } from './mock-database';
import { mockEvent } from './mock-event';
import { mockFolders, mockTags } from './organizer';

type Item = Generated.ActionItemView;
type View = Generated.SavedView;

const people = personOptions(mockPeople);
const me = people.find((person) => person.isMe)?.id ?? null;
let items: Item[] = fixtureActionItems(fixtureMeetings, mockPeople);
let views: View[] = [];
let created = 0;

export const events = {
  actionItemChanged: mockEvent<Generated.ActionItemChanged>(
    'plugin:action-items:action-item-changed',
  ),
} satisfies typeof Generated.events;

function list(filter: Generated.ActionItemFilter, now = new Date()): Item[] {
  return items
    .filter((item) =>
      matches({ item, companies: rowCompanies(item, fixtureMeetings, mockPeople) }, filter, {
        me,
        now,
      }),
    )
    .sort(byDue);
}

const notFound = (id: string) =>
  failure<never>({ code: 'not_found', message: `action item ${id} was not found` });
const invalid = (message: string) => failure<never>({ code: 'invalid', message });

/** Applies `edit` to `ids`; the number changed. */
function apply(ids: readonly Generated.ActionItemId[], edit: Generated.ActionItemEdit): number {
  const now = new Date();
  let changed = 0;
  items = items.map((item) => {
    if (!ids.includes(item.id)) return item;
    changed += 1;
    return edited(item, edit, { now, people });
  });
  mockDatabase.touch(['action_items']);
  return changed;
}

function newItem(input: Generated.NewActionItem): Item {
  const now = new Date().toISOString();
  created += 1;
  const person = people.find((candidate) => candidate.id === input.assigneePersonId) ?? null;
  return {
    id: `0199c6a0-0000-7000-8000-${String(created).padStart(12, '0')}` as Generated.ActionItemId,
    meetingId: input.meetingId,
    title: input.title.trim(),
    description: input.description.trim(),
    assigneePersonId: person?.id ?? null,
    dueAt: input.due === null ? null : localNoon(input.due),
    status: 'open',
    priority: input.priority,
    sourceSegmentId: null,
    completedAt: null,
    createdBy: 'user',
    createdAt: now,
    updatedAt: now,
    meetingTitle: null,
    meetingHeldAt: null,
    assigneeName: person?.name ?? null,
    assigneeIsMe: person?.isMe ?? false,
    sourceStart: null,
  };
}

function saveView(id: Generated.ActionItemViewId | null, input: Generated.SavedViewInput) {
  const name = input.name.trim();
  if (name === '') return invalid('name cannot be empty');
  const clash = views.some(
    (view) => view.id !== id && view.name.toLowerCase() === name.toLowerCase(),
  );
  if (clash)
    return failure<never>({ code: 'conflict', message: 'a view with this name already exists' });
  const now = new Date().toISOString();
  const existing = views.find((view) => view.id === id);
  const view: View = existing
    ? { ...existing, name, filter: input.filter, updatedAt: now }
    : {
        id: `0199c6a1-0000-7000-8000-${String(views.length + 1).padStart(12, '0')}` as Generated.ActionItemViewId,
        name,
        filter: input.filter,
        position: views.length,
        createdAt: now,
        updatedAt: now,
      };
  views = existing
    ? views.map((candidate) => (candidate.id === id ? view : candidate))
    : [...views, view];
  mockDatabase.touch(['action_item_views']);
  return ok(view);
}

export const commands = {
  async list(filter) {
    await latency();
    return ok(list(filter));
  },
  async today() {
    await latency();
    const mine = {
      assignee: { kind: 'me' },
      status: 'open',
      meetingId: null,
      tagId: null,
      companyId: null,
      folderId: null,
      dateRange: null,
    } as const;
    return ok({
      overdue: list({ ...mine, due: 'overdue' }),
      dueToday: list({ ...mine, due: 'today' }),
    });
  },
  async create(input) {
    await latency();
    if (input.title.trim() === '') return invalid('title cannot be empty');
    const item = newItem(input);
    items = [...items, item];
    mockDatabase.touch(['action_items']);
    return ok(item);
  },
  async update(id, edit) {
    await latency();
    if (edit.field === 'title' && edit.title.trim() === '') return invalid('title cannot be empty');
    if (apply([id], edit) === 0) return notFound(id);
    const item = items.find((candidate) => candidate.id === id);
    return item === undefined ? notFound(id) : ok(item);
  },
  async bulkUpdate(ids, edit) {
    await latency();
    if (edit.field === 'title' || edit.field === 'description') {
      return invalid("edit cannot change many items' wording at once");
    }
    return ok(apply(ids, edit));
  },
  async delete(ids) {
    await latency();
    const before = items.length;
    items = items.filter((item) => !ids.includes(item.id));
    mockDatabase.touch(['action_items']);
    return ok(before - items.length);
  },
  async markAllMineDone() {
    await latency();
    const mine = list({
      assignee: { kind: 'me' },
      status: 'open',
      meetingId: null,
      tagId: null,
      companyId: null,
      folderId: null,
      due: null,
      dateRange: null,
    });
    return ok(
      apply(
        mine.map((item) => item.id),
        { field: 'status', status: 'done' },
      ),
    );
  },
  async filterOptions() {
    await latency();
    return ok({
      people,
      companies: fixtureCompanies(fixtureMeetings),
      tags: mockTags.list().map(({ id, name, color }) => ({ id, name, color })),
      folders: mockFolders.tree().entries.map(({ folder }) => ({
        id: folder.id,
        name: folder.name,
        parentId: folder.parentId,
      })),
    });
  },
  async views() {
    await latency();
    return ok([...views]);
  },
  async createView(view) {
    await latency();
    return saveView(null, view);
  },
  async updateView(id, view) {
    await latency();
    return saveView(id, view);
  },
  async deleteView(id) {
    await latency();
    if (!views.some((view) => view.id === id))
      return failure<never>({ code: 'not_found', message: `view ${id} was not found` });
    views = views.filter((view) => view.id !== id);
    mockDatabase.touch(['action_item_views']);
    return ok(null);
  },
} satisfies typeof Generated.commands;
