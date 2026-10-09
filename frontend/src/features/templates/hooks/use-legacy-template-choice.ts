import { asMeetingId } from '@/features/meetings';

import { useMeetingTemplate, useSetMeetingTemplate, useTemplates } from '../queries';
import { asTemplateId, GENERAL_TEMPLATE_ID } from '../types';

/** A template as the legacy summary panel lists it. */
export interface LegacyTemplateOption {
  id: string;
  name: string;
  description: string;
}

/** The legacy summary panel's template state, backed by the templates plugin. */
export interface LegacyTemplateChoice {
  availableTemplates: LegacyTemplateOption[];
  /** The meeting's template, which the legacy summary is generated with. */
  selectedTemplate: string;
  handleTemplateSelection: (templateId: string, templateName: string) => void;
}

/**
 * The legacy meeting panels' template picker (it replaced the JSON-file
 * `useTemplates`): the stored templates, the meeting's resolved template,
 * and choosing one saves it as the meeting's template. NOT-02 removes it.
 */
export function useLegacyTemplateChoice(meetingId: string): LegacyTemplateChoice {
  const meeting = asMeetingId(meetingId);
  const templates = useTemplates();
  const choice = useMeetingTemplate(meeting);
  const set = useSetMeetingTemplate(meeting);
  return {
    availableTemplates: (templates.data ?? []).map((template) => ({
      id: template.id,
      name: template.content.name,
      description: template.content.description,
    })),
    selectedTemplate: choice.data?.template.id ?? GENERAL_TEMPLATE_ID,
    handleTemplateSelection: (templateId) => {
      set.mutate(asTemplateId(templateId));
    },
  };
}
