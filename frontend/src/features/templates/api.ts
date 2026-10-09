/**
 * Typed calls to the `templates` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts`.
 */
import type { MeetingId } from '@/features/meetings';
import { commands } from '@/lib/ipc/generated/templates';
import { unwrap } from '@/lib/ipc/result';

import type {
  Template,
  TemplateChoice,
  TemplateContent,
  TemplateId,
  TemplatePreview,
} from './types';

export const templatesApi = {
  /** Every template, built-in and the user's, by name. */
  list: (): Promise<Template[]> => unwrap(commands.list()),
  /** One template; rejects with `not_found`. */
  get: (id: TemplateId): Promise<Template> => unwrap(commands.get(id)),
  /** Saves a new template; rejects with `invalid` naming the broken rule. */
  create: (content: TemplateContent): Promise<Template> => unwrap(commands.create(content)),
  /** Replaces a user template's content; rejects a built-in with `conflict`. */
  update: (id: TemplateId, content: TemplateContent): Promise<Template> =>
    unwrap(commands.update(id, content)),
  /** An editable copy, named "<name> (copy)". */
  duplicate: (id: TemplateId): Promise<Template> => unwrap(commands.duplicate(id)),
  /** Deletes a user template; rejects a built-in with `conflict`. */
  delete: (id: TemplateId): Promise<null> => unwrap(commands.delete(id)),
  /** Saves a template file (Deino's or the pre-NOT-04 format) as a new template. */
  importJson: (json: string): Promise<Template> => unwrap(commands.importJson(json)),
  /** The template as a shareable JSON file. */
  exportJson: (id: TemplateId): Promise<string> => unwrap(commands.exportJson(id)),
  /** A template rendered against a meeting, without an LLM call. */
  preview: (content: TemplateContent, meeting: MeetingId): Promise<TemplatePreview> =>
    unwrap(commands.preview(content, meeting)),
  /** The template a meeting gets, and why. */
  meetingTemplate: (meeting: MeetingId): Promise<TemplateChoice> =>
    unwrap(commands.meetingTemplate(meeting)),
  /** Chooses a meeting's template, or `null` for automatic selection. */
  setMeetingTemplate: (meeting: MeetingId, template: TemplateId | null): Promise<TemplateChoice> =>
    unwrap(commands.setMeetingTemplate(meeting, template)),
};
