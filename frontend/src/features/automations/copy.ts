/** Visible strings of the automations feature. */
import type {
  AutomationRunStatus,
  AutomationStepStatus,
  ExportFormat,
  PayloadFields,
  Trigger,
} from './types';

export const automationsCopy = {
  title: 'Automations',
  description: 'Steps that run after meetings like these: tag, file, export, notify, run a script.',
  create: 'New automation',
  emptyTitle: 'No automations yet',
  emptyBody: 'Create one to tag, file, export or script meetings as they finish.',
  pick: 'Pick an automation, or create one.',
  untitled: 'New automation',
  off: 'Off',
  manualOnly: 'Manual',
  tabs: { builder: 'Builder', matching: 'Matching meetings' },
  shellToggle: 'Allow shell scripts',
  shellToggleHint: 'Scripts run with your permissions. Off until you turn it on.',
} as const;

export const builderCopy = {
  name: 'Name',
  namePlaceholder: 'Weekly recap',
  enabled: 'On',
  autoRun: 'Run automatically after matching meetings',
  stopOnFailure: 'Stop at the first failed step',
  when: 'When',
  everyMeeting: 'Every meeting (no conditions)',
  steps: 'Steps',
  noSteps: 'Add the first step.',
  addStep: 'Add step',
  wip: 'WIP',
  moveUp: 'Move step up',
  moveDown: 'Move step down',
  removeStep: 'Remove step',
  save: 'Save',
  saved: 'Saved',
  test: 'Test on latest meeting',
  tested: 'Running on the latest meeting…',
  duplicate: 'Duplicate',
  delete: 'Delete',
  choose: 'Choose…',
  insight: 'Insight',
  tag: 'Tag',
  folder: 'Folder',
  format: 'Format',
  exportFolder: '/Users/you/Notes/Meetings',
  message: 'Message (optional)',
  command: 'Command',
  commandPlaceholder: 'jq .meeting.title >> ~/meetings.log',
  payload: 'Payload includes',
  shellOff: 'Shell scripts are off. Turn on “Allow shell scripts” above for this step to run.',
} as const;

/** Payload field labels. */
export const payloadCopy: Record<keyof PayloadFields, string> = {
  notes: 'Notes',
  actionItems: 'Action items',
  transcript: 'Transcript',
  insights: 'Insights',
  attendees: 'Attendees',
  link: 'Link',
  audioPath: 'Audio path',
};

/** Export format labels. */
export const formatCopy: Record<ExportFormat, string> = {
  markdown: 'Markdown',
  text: 'Plain text',
  json: 'JSON',
  srt: 'SRT subtitles',
  vtt: 'WebVTT subtitles',
  docx: 'Word',
  pdf: 'PDF',
};

export const runCopy = {
  never: 'Not run',
  status: {
    armed: 'Armed',
    pending: 'Queued',
    done: 'Done',
    failed: 'Failed',
  } satisfies Record<AutomationRunStatus, string>,
  step: {
    pending: 'Waiting',
    done: 'Done',
    failed: 'Failed',
    skipped: 'Skipped',
  } satisfies Record<AutomationStepStatus, string>,
  trigger: {
    pipeline: 'after processing',
    tag_applied: 'when tagged',
    manual: 'by hand',
  } satisfies Record<Trigger['kind'], string>,
  run: 'Run',
  rerun: 'Rerun',
  runSelected: (count: number) => `Run on ${String(count)} selected`,
  noMatches: 'No finished meeting matches these conditions yet.',
  selectAll: 'Select all',
  menu: 'Automations',
  menuEmpty: 'No automations yet.',
  more: 'More actions',
  notMatching: 'conditions don’t match',
} as const;
