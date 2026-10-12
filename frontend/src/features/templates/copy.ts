import type { Audience, ChoiceReason, SectionFormat } from './types';

/** Every visible string of the templates feature. */
export const templatesCopy = {
  defaultLabel: 'Default template',
  defaultDescription:
    'Used when you have not picked one and no auto rule matches. Auto lets AI suggest one after the meeting.',
  auto: 'Auto',
  yours: 'Your templates',
  builtIn: 'Built-in',
  newTemplate: 'New template',
  import: 'Import…',
  importTitle: 'Import a template',
  importBody:
    'Choose a template file, or paste its JSON. Files from older Deino versions work too.',
  importFile: 'Template file',
  importJson: 'Template JSON',
  importAction: 'Import',
  exported: 'Template copied as JSON',
  emptyTitle: 'No templates of your own yet',
  emptyBody: 'Duplicate a built-in to adapt it, or start from scratch.',
  actionsFor: (name: string) => `Actions for ${name}`,
  duplicate: 'Duplicate',
  export: 'Copy as JSON',
  delete: 'Delete',
  deleteTitle: 'Delete this template?',
  deleteBody: 'Meetings that use it go back to automatic selection.',
  cancel: 'Cancel',
  sectionCount: (count: number) => `${count} ${count === 1 ? 'section' : 'sections'}`,
  ruleCount: (count: number) =>
    count === 0 ? '' : ` · ${count} auto ${count === 1 ? 'rule' : 'rules'}`,

  back: 'Templates',
  readOnlyTitle: 'Built-in templates are read-only',
  readOnlyBody: 'Duplicate it to change the sections, instructions or rules.',
  name: 'Name',
  namePlaceholder: 'Template name',
  description: 'Description',
  descriptionHint: 'One line. Shown in pickers, and used by AI to choose templates.',
  meetingContext: 'Meeting context',
  meetingContextHint: 'Who the meeting is with and what the notes are for.',
  sections: 'Sections',
  sectionsHint: 'In order. Drag to reorder, or use Alt ↑ and Alt ↓.',
  sectionTitle: 'Section title',
  instructions: 'Instructions',
  instructionsPlaceholder: 'What belongs in this section',
  format: 'Format',
  moveUp: 'Move section up',
  moveDown: 'Move section down',
  removeSection: 'Remove section',
  dragSection: (title: string) => `Reorder ${title}`,
  addSection: 'Add section',
  newSection: 'New section',
  save: 'Save',
  saving: 'Saving…',

  rules: 'Auto rules',
  rulesHint: 'Pick this template by itself when every condition of a rule holds.',
  addRule: 'Add rule',
  removeRule: 'Remove rule',
  ruleLabel: 'Label',
  ruleLabelPlaceholder: 'External sales call',
  titlePattern: 'Title matches',
  titlePatternPlaceholder: 'discovery|intro',
  audience: 'Attendees',
  anyAudience: 'Anyone',
  domains: 'From domains',
  domainsPlaceholder: 'acme.com, globex.com',
  count: 'Attendee count',
  min: 'Min',
  max: 'Max',

  preview: 'Preview',
  previewHint: 'What the AI receives for a past meeting. Nothing is sent.',
  previewMeeting: 'Meeting',
  pickMeeting: 'Choose a meeting',
  renderPreview: 'Render',

  picker: 'Template',
  pickerLabel: 'Note template',
  pickerAuto: 'Back to automatic',
  regenerateTitle: (name: string) => `Notes were written before you chose ${name}.`,
  regenerate: 'Regenerate notes',
  dismiss: 'Not now',

  formats: {
    bullets: 'Bullets',
    paragraph: 'Paragraph',
    table: 'Table',
    todo: 'To-dos',
  } satisfies Record<SectionFormat, string>,
  audiences: {
    internal: 'Colleagues only',
    external: 'Someone external',
  } satisfies Record<Audience, string>,
} as const;

/** Why a meeting has its template, in a few words. */
export function reasonText(reason: ChoiceReason): string {
  switch (reason.kind) {
    case 'chosen':
      return 'Chosen for this meeting';
    case 'rule':
      return `Matched rule: ${reason.rule}`;
    case 'classified':
      return `Suggested by AI: ${reason.explanation}`;
    case 'default':
      return 'Your default template';
    case 'fallback':
      return 'Automatic';
  }
}
