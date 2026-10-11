import type { ExportFormat, SectionKey } from './types';

/** Every visible string of the export feature. */
export const exportCopy = {
  share: 'Share',
  shareLabel: 'Share and export',
  exportAs: 'Export…',
  copyRich: 'Copy as rich text',
  copyMarkdown: 'Copy as Markdown',
  copyTranscript: 'Copy transcript',
  email: 'Email notes',
  copied: 'Copied',
  copiedRich: 'Notes copied with formatting',
  copiedMarkdown: 'Notes copied as Markdown',
  copiedTranscript: 'Transcript copied',
  emailOpened: 'Draft opened in Mail',
  saved: (fileName: string) => `Saved ${fileName}`,

  dialogTitle: 'Export meeting',
  dialogDescription: 'Choose a format and what to include. Nothing leaves your Mac.',
  formatLabel: 'Format',
  includeLabel: 'Include',
  fixedContent: (format: string) => `${format} always contains the whole meeting.`,
  transcriptOnly: (format: string) => `${format} is the transcript with speakers.`,
  previewLabel: 'Preview',
  previewFrame: 'Document preview',
  previewLoading: 'Rendering the preview…',
  previewError: 'The preview could not be rendered.',
  cancel: 'Cancel',
  confirm: 'Export…',
  exporting: 'Exporting…',

  settingsTitle: 'Export',
  autoTitle: 'Auto-export',
  autoDescription:
    'Write every meeting as a Markdown note into a folder, such as an Obsidian vault, once it is processed.',
  autoToggle: 'Export after every meeting',
  autoToggleHint: 'Runs as the last step of processing.',
  folderLabel: 'Folder',
  folderNone: 'No folder chosen',
  folderChoose: 'Choose…',
  folderChange: 'Change…',
  fileNameLabel: 'File name',
  fileNameHint: 'Use {date}, {time}, {title}, {yyyy}, {mm} and {dd}.',
  subfolderLabel: 'Subfolders',
  subfolderHint: 'Folders inside the export folder, separated by /. Leave empty for none.',
  obsidianLabel: 'Obsidian mode',
  obsidianHint: 'People become [[links]] with their own pages; tags become #tags.',
  sidecarLabel: 'Transcript in its own note',
  sidecarHint: 'Keeps meeting notes short and links to the transcript.',
  overwriteLabel: 'Update notes when a meeting is processed again',
  overwriteHint: 'Deino only ever rewrites notes it wrote itself.',
  sectionsLabel: 'Each note includes',

  accountTitle: 'Export everything',
  accountDescription:
    'A zip of every meeting as Markdown and JSON, with tables of meetings, action items and people.',
  accountAudio: 'Include recordings',
  accountAudioHint: 'Makes the archive much larger.',
  accountStart: 'Export all meetings…',
  accountCancel: 'Cancel',
  accountProgress: (done: number, total: number) =>
    `Exporting ${String(done)} of ${String(total)} meetings…`,
  accountFinished: (fileName: string) => `Saved ${fileName}.`,
  accountCancelled: 'The export was cancelled.',
  accountFailed: 'The export stopped.',
} as const;

/** Each format's name and what it is for, in the order the dialog lists them. */
export const formatCopy: Record<ExportFormat, { label: string; hint: string }> = {
  markdown: { label: 'Markdown', hint: 'Obsidian and any notes app' },
  pdf: { label: 'PDF', hint: 'Print-ready, to share as is' },
  docx: { label: 'Word', hint: 'Edit in Word or Pages' },
  text: { label: 'Plain text', hint: 'No formatting at all' },
  json: { label: 'JSON', hint: 'Everything, for scripts' },
  srt: { label: 'SRT', hint: 'Subtitles for the recording' },
  vtt: { label: 'WebVTT', hint: 'Subtitles for the web' },
};

/** Each section switch's label. */
export const sectionCopy: Record<SectionKey, string> = {
  attendees: 'Attendees',
  notes: 'Notes',
  myNotes: 'My notes',
  actionItems: 'Action items',
  insights: 'Insights',
  transcript: 'Transcript',
};
