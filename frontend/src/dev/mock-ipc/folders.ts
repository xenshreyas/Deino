/**
 * Mock of `@/lib/ipc/generated/folders`, over the in-memory folders in
 * `organizer.ts`. `satisfies` keeps it in step with the Rust commands.
 */
import type * as Generated from '@/lib/ipc/generated/folders';

import { latency } from './latency';
import { ok } from './mock-database';
import { mockFolders } from './organizer';

export const commands = {
  async tree() {
    await latency();
    return ok(mockFolders.tree());
  },
  async get(id) {
    await latency();
    return mockFolders.get(id);
  },
  async create(folder) {
    await latency();
    return mockFolders.create(folder, new Date());
  },
  async rename(id, name) {
    await latency();
    return mockFolders.rename(id, name, new Date());
  },
  async setIcon(id, icon) {
    await latency();
    return mockFolders.update(id, { icon }, new Date());
  },
  async setDescription(id, description) {
    await latency();
    return mockFolders.update(id, { description: description.trim() }, new Date());
  },
  async moveTo(id, parentId) {
    await latency();
    return mockFolders.moveTo(id, parentId, new Date());
  },
  async delete(id) {
    await latency();
    return mockFolders.delete(id);
  },
  async meetings(id) {
    await latency();
    return ok(mockFolders.meetingsIn(id));
  },
  async unfiled() {
    await latency();
    return ok(mockFolders.unfiled());
  },
  async foldersOf(meetingId) {
    await latency();
    return ok(mockFolders.foldersOf(meetingId));
  },
  async addMeeting(folderId, meetingId) {
    await latency();
    return mockFolders.addMeeting(folderId, meetingId);
  },
  async removeMeeting(folderId, meetingId) {
    await latency();
    return mockFolders.removeMeeting(folderId, meetingId);
  },
  async rules(folderId) {
    await latency();
    return ok(mockFolders.rulesOf(folderId));
  },
  async addRule(folderId, rule) {
    await latency();
    return mockFolders.addRule(folderId, rule, new Date());
  },
  async removeRule(id) {
    await latency();
    return mockFolders.removeRule(id);
  },
} satisfies typeof Generated.commands;
