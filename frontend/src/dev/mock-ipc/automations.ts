/**
 * Mock of `@/lib/ipc/generated/automations`, over the in-memory automations
 * in `mock-automations.ts`: the step catalogue (provider steps WIP),
 * definitions, the meeting menu, matching meetings and runs that finish a
 * moment later.
 */
import type * as Generated from '@/lib/ipc/generated/automations';

import { ok } from './command-result';
import { mockAutomations } from './database';
import { latency } from './latency';

/** The catalogue as the app's registry reports it (local steps ready). */
const CATALOG: Generated.StepInfo[] = [
  ['generate_insight', 'Generate insight', 'Run a custom insight; later steps get its answers.'],
  ['add_tag', 'Add tag', 'Tag the meeting.'],
  ['add_to_folder', 'Add to folder', 'File the meeting in a folder.'],
  ['export_file', 'Export file', 'Write the meeting as a file into a folder.'],
  ['notify', 'Notification', 'Show a macOS notification.'],
  ['run_shell', 'Run shell script', 'Run a command with the meeting as JSON on stdin.'],
  ['webhook', 'Webhook', 'Send the meeting to a URL.'],
  ['slack', 'Slack', 'Post the notes to a channel.'],
  ['notion', 'Notion', 'Create a page or database row.'],
  ['tasks', 'Task tools', 'Create tasks in Linear, Todoist and others.'],
  ['crm', 'CRM', 'Update HubSpot, Salesforce or Attio.'],
  ['document', 'Google Docs or Apple Notes', 'Create a document.'],
  ['email', 'Email', 'Draft or send a recap email.'],
].map(([kind, label, description], index) => ({
  kind: kind as Generated.ActionKind,
  label: label ?? '',
  description: description ?? '',
  availability: index < 6 ? 'ready' : 'wip',
}));

export const commands = {
  async catalog() {
    await latency();
    return ok(CATALOG);
  },
  async list() {
    await latency();
    return ok(mockAutomations.list());
  },
  async get(id) {
    await latency();
    return mockAutomations.get(id);
  },
  async create(input) {
    await latency();
    return mockAutomations.save(null, input, new Date());
  },
  async update(id, input) {
    await latency();
    return mockAutomations.save(id, input, new Date());
  },
  async delete(id) {
    await latency();
    return mockAutomations.delete(id);
  },
  async duplicate(id) {
    await latency();
    return mockAutomations.duplicate(id, new Date());
  },
  async forMeeting(meetingId) {
    await latency();
    return mockAutomations.forMeeting(meetingId);
  },
  async matching(id) {
    await latency();
    return mockAutomations.matching(id);
  },
  async run(id, meetingIds) {
    await latency();
    return mockAutomations.run(id, meetingIds, new Date());
  },
  async testLatest(id) {
    await latency();
    return mockAutomations.testLatest(id, new Date());
  },
} satisfies typeof Generated.commands;
