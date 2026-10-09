/**
 * Mock of `@/lib/ipc/generated/tags`, over the in-memory tags in
 * `organizer.ts`. Applying a tag announces `MeetingTagged`, like Rust.
 */
import type * as Generated from '@/lib/ipc/generated/tags';

import { latency } from './latency';
import { ok } from './mock-database';
import { mockEvent } from './mock-event';
import { mockTags } from './organizer';

export const events = {
  meetingTagged: mockEvent<Generated.MeetingTagged>('plugin:tags:meeting-tagged'),
} satisfies typeof Generated.events;

export const commands = {
  async list() {
    await latency();
    return ok(mockTags.list());
  },
  async create(tag) {
    await latency();
    return mockTags.create(tag, new Date());
  },
  async update(id, update) {
    await latency();
    return mockTags.update(id, update, new Date());
  },
  async delete(id) {
    await latency();
    return mockTags.delete(id);
  },
  async tagsOf(meetingId) {
    await latency();
    return ok(mockTags.tagsOf(meetingId));
  },
  async links() {
    await latency();
    return ok(mockTags.allLinks());
  },
  async apply(meetingId, tagId) {
    await latency();
    const result = mockTags.apply(meetingId, tagId);
    if (result.status === 'ok')
      void events.meetingTagged.emit({ meetingId, tagId, appliedBy: 'user' });
    return result;
  },
  async remove(meetingId, tagId) {
    await latency();
    return mockTags.remove(meetingId, tagId);
  },
} satisfies typeof Generated.commands;
