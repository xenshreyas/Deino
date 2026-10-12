/**
 * The templates feature (NOT-04): the template library and editor
 * (Settings → Templates), the meeting's template menu in the notes' toolbar,
 * and the notepad's `/template` command.
 * Other code imports from here only.
 */
export { templatesApi } from './api';
export {
  MeetingTemplateMenu,
  type MeetingTemplateMenuProps,
  TemplateSwitchedPrompt,
  type TemplateSwitchedPromptProps,
} from './components/meeting-template-picker';
export { TemplatesPage } from './components/templates-page';
export { reasonText, templatesCopy } from './copy';
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
