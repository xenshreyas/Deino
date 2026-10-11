/**
 * Mock of `@/lib/ipc/generated/insights`, over the in-memory insights in
 * `mock-insights.ts`: the gallery, definitions, the tab and the table, runs
 * that answer a moment later, and a CSV "saved" under the insight's name.
 */
import type * as Generated from '@/lib/ipc/generated/insights';

import { ok } from './command-result';
import { mockInsights } from './database';
import { latency } from './latency';
import { MOCK_INSIGHT_TEMPLATES } from './mock-insight-templates';

export const commands = {
  async templates() {
    await latency();
    return MOCK_INSIGHT_TEMPLATES;
  },
  async list() {
    await latency();
    return ok(mockInsights.list());
  },
  async get(id) {
    await latency();
    return mockInsights.get(id);
  },
  async create(input) {
    await latency();
    return mockInsights.save(null, input, new Date());
  },
  async update(id, input) {
    await latency();
    return mockInsights.save(id, input, new Date());
  },
  async delete(id) {
    await latency();
    return mockInsights.delete(id);
  },
  async forMeeting(meetingId) {
    await latency();
    return mockInsights.forMeeting(meetingId);
  },
  async table(id) {
    await latency();
    return mockInsights.table(id);
  },
  async candidates(id) {
    await latency();
    return mockInsights.candidates(id);
  },
  async run(id, meetingIds) {
    await latency();
    return mockInsights.run(id, meetingIds, new Date());
  },
  async exportCsv(id) {
    await latency();
    const found = mockInsights.get(id);
    if (found.status === 'error') return found;
    return ok({ fileName: `${found.data.name}.csv` });
  },
} satisfies typeof Generated.commands;
