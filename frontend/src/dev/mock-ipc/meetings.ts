/**
 * Mock of `@/lib/ipc/generated/meetings`, served from the fixture meetings.
 * `satisfies` keeps it in step with the Rust commands: a changed signature
 * fails `pnpm typecheck` here. The Meetings page's commands (ORG-07) go to
 * `mock-meetings-list.ts`.
 */
import type * as Generated from '@/lib/ipc/generated/meetings';

import { mockDatabase } from './database';
import { latency } from './latency';
import { ok } from './mock-database';
import { mockMeetingsList as list } from './mock-meetings-list';
import { mockFolders, mockTags } from './organizer';

/** Runs `apply` on each ID and counts them. */
function each(ids: readonly Generated.MeetingId[], apply: (id: Generated.MeetingId) => void) {
  for (const id of ids) apply(id);
  return ok({ changed: ids.length });
}

export const commands = {
  async list() {
    await latency();
    return ok(mockDatabase.meetings());
  },
  async get(id) {
    await latency();
    return mockDatabase.meeting(id);
  },
  async rename(id, title) {
    await latency();
    return mockDatabase.rename(id, title, new Date());
  },
  async page(filter, after, size) {
    await latency();
    return ok(list.page(filter, after, size));
  },
  async matchingIds(filter) {
    await latency();
    return ok(list.matching(filter).map((meeting) => meeting.id));
  },
  async views() {
    await latency();
    return ok(list.views());
  },
  async createView(view) {
    await latency();
    return list.saveView(null, view);
  },
  async updateView(id, view) {
    await latency();
    return list.saveView(id, view);
  },
  async deleteView(id) {
    await latency();
    return list.deleteView(id);
  },
  async trash(ids) {
    await latency();
    return ok(list.trash(ids));
  },
  async restore(ids) {
    await latency();
    return ok(list.restore(ids));
  },
  async trashed() {
    await latency();
    return ok(list.trashedMeetings());
  },
  async deleteForever(ids) {
    await latency();
    return ok(list.deleteForever(ids));
  },
  async tagMany(ids, tag) {
    await latency();
    return each(ids, (id) => void mockTags.apply(id, tag));
  },
  async untagMany(ids, tag) {
    await latency();
    return each(ids, (id) => void mockTags.remove(id, tag));
  },
  async fileMany(ids, folder) {
    await latency();
    return each(ids, (id) => void mockFolders.addMeeting(folder, id));
  },
  async merge(ids) {
    await latency();
    return list.merge(ids);
  },
} satisfies typeof Generated.commands;
