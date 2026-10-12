/** Visible strings of the automations feature. */
import type {
  ActionKind,
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
  draft: 'Draft',
} as const;

export const builderCopy = {
  name: 'Name',
  namePlaceholder: 'Name this automation',
  enabled: 'On',
  autoRun: 'Run automatically after matching meetings',
  stopOnFailure: 'Stop at the first failed step',
  when: 'When',
  everyMeeting: 'Every meeting (no conditions)',
  steps: 'Steps',
  noSteps: 'Add the first step.',
  addStep: 'Add step',
  wip: 'WIP',
  comingLater: 'Coming later',
  needs: {
    generate_insight: 'Create an insight first.',
    add_tag: 'Create a tag first.',
    add_to_folder: 'Create a folder first.',
    run_shell: 'Turn on “Allow shell scripts” at the top of the page first.',
    webhook: 'Add a webhook in Settings → Developer first.',
  } satisfies Partial<Record<ActionKind, string>>,
  moveUp: 'Move step up',
  moveDown: 'Move step down',
  removeStep: 'Remove step',
  save: 'Save',
  saved: 'Saved',
  test: 'Test on latest meeting',
  tested: 'Running on the latest meeting…',
  more: 'More actions',
  duplicate: 'Duplicate',
  delete: 'Delete',
  choose: 'Choose…',
  insight: 'Insight',
  tag: 'Tag',
  folder: 'Folder',
  webhook: 'Webhook',
  format: 'Format',
  chooseFolder: 'Choose folder…',
  changeFolder: 'Change',
  exportFolder: 'Export folder',
  message: 'Message (optional)',
  command: 'Command',
  commandPlaceholder: 'jq .meeting.title >> ~/meetings.log',
  payload: 'Payload includes',
  shellOff:
    'Shell scripts are off, so this step won’t run. Turn on “Allow shell scripts” at the top of the page.',
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
  recent: 'Recent runs',
  noRuns: 'No runs yet. Test it on the latest meeting, or run it from Matching meetings.',
  openMeeting: (title: string) => `Open ${title}`,
  untitledMeeting: 'Untitled meeting',
} as const;
