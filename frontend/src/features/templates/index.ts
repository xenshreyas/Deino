/**
 * The templates feature (NOT-04): the template library and editor
 * (Settings → Templates), the meeting's template picker on the Notes tab,
 * the notepad's `/template` command, and the legacy summary panel's bridge.
 * Other code imports from here only.
 */
export { templatesApi } from './api';
export {
  MeetingTemplatePicker,
  type MeetingTemplatePickerProps,
} from './components/meeting-template-picker';
export { TemplatesPage } from './components/templates-page';
export { reasonText, templatesCopy } from './copy';
export {
  type LegacyTemplateChoice,
  type LegacyTemplateOption,
  useLegacyTemplateChoice,
} from './hooks/use-legacy-template-choice';
export {
  meetingTemplateQuery,
  templateKeys,
  templateListQuery,
  useMeetingTemplate,
  useSetMeetingTemplate,
  useTemplates,
} from './queries';
export { TemplateSlashList, type TemplateSlashListProps } from './slash/template-slash-list';
export {
  scaffoldContent,
  type ScaffoldEditor,
  type ScaffoldNode,
  TEMPLATE_COMMAND,
  type TemplateSlashDeps,
  type TemplateSlashSuggestion,
  templateSlashSuggestion,
} from './slash/template-slash';
export {
  asTemplateId,
  GENERAL_TEMPLATE_ID,
  type Template,
  type TemplateChoice,
  type TemplateContent,
  type TemplateId,
} from './types';
