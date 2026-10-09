/**
 * What the `/template` command needs from the app: the cached template list
 * (kept warm while the notepad is open) and a way to set the meeting's
 * template. On the record screen, choosing a template creates the draft.
 */
import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import {
  type Template,
  type TemplateId,
  templateListQuery,
  templatesApi,
  useTemplates,
} from '@/features/templates';
import { IpcError } from '@/lib/ipc/result';

import { notepadCopy } from '../copy';
import { createMenuStore, type MenuStore } from '../editor/menu-store';
import type { TemplateCommandOptions } from '../editor/template-command';
import type { NoteTarget } from '../target';

/** Sets the template of the target's meeting; a failure becomes a toast. */
async function setTemplate(target: NoteTarget, template: TemplateId): Promise<void> {
  try {
    const { meetingId } = await target.resolve();
    await templatesApi.setMeetingTemplate(meetingId, template);
  } catch (error) {
    toast.error(notepadCopy.templateFailedTitle, { description: IpcError.from(error).message });
  }
}

/** The command's menu store and actions for one editor. */
export function useTemplateCommand(
  target: NoteTarget,
): TemplateCommandOptions & { menu: MenuStore<Template> } {
  const client = useQueryClient();
  useTemplates();
  const [menu] = useState(createMenuStore<Template>);
  const [deps] = useState(() => ({
    templates: () => client.getQueryData(templateListQuery().queryKey) ?? [],
    setMeetingTemplate: (template: TemplateId) => {
      void setTemplate(target, template);
    },
  }));
  return { menu, deps };
}
