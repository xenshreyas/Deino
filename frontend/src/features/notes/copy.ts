/** User-visible strings of the enhanced notes (NOT-02) and citations (NOT-03). */
import type { RegenerateReason } from './types';

export const notesCopy = {
  enhanced: 'Enhanced',
  editorLabel: 'Enhanced notes',
  enhance: 'Enhance',
  regenerate: 'Regenerate',
  enhanceHint: 'Enhance notes',
  regenerateHint: 'Regenerate notes',
  writing: 'Writing your notes…',
  rewriting: 'Writing new notes…',
  emptyTitle: 'No enhanced notes yet',
  emptyBody:
    'Deino turns your notes and the transcript into finished notes, keeping every line you typed.',
  failedTitle: 'The notes could not be written',
  loadFailed: 'The notes could not be opened',
  openSettings: 'Settings → Notes',
  tryAgain: 'Try again',
  confirmTitle: 'Replace your edits?',
  confirmBody:
    'Regenerating writes new notes from the transcript and your notes. The version you edited stays in the version history.',
  confirmAction: 'Regenerate',
  cancel: 'Cancel',
  bannerLead: 'Things changed since these notes were written',
  dismiss: 'Not now',
  versionsLabel: 'Version history',
  version: (n: number) => `Version ${String(n)}`,
  edited: 'Edited',
  newest: 'Newest',
  viewingOld: (n: number, total: number) => `Version ${String(n)} of ${String(total)}`,
  restore: 'Restore',
  backToNewest: 'Back to newest',
  copyLabel: 'Copy notes',
  copyMarkdown: 'Copy as Markdown',
  copyRichText: 'Copy as rich text',
  copied: 'Copied',
  copyFailed: 'The notes could not be copied',
  contents: 'On this page',
  sourcesLabel: 'Show sources',
  sourcesTitle: 'From the transcript',
  sourcesLoading: 'Finding the moment…',
  sourcesGone: 'This part of the transcript was deleted.',
  play: 'Play from here',
  noRecording: 'No recording to play',
  showInTranscript: 'Show in transcript',
  saveFailed: 'Your edit was not saved yet; retrying',
  readingTranscript: 'Reading the transcript…',
  readingParts: (done: number, total: number) =>
    `Reading the transcript in ${String(total)} parts · ${String(done)} of ${String(total)} done`,
  merging: 'Merging the parts…',
  writingSection: (heading: string) => (heading === '' ? 'Writing…' : `Writing · ${heading}`),
  retrying: 'The first answer could not be used. Asking again…',
  saving: 'Saving…',
  stop: 'Stop',
  stopHint: 'Stop writing; nothing is saved',
  lockedHint: 'You can edit once the notes are written',
  writingBelow: 'Writing below',
  written: (kept: number) =>
    kept === 0
      ? 'Notes written'
      : kept === 1
        ? 'Notes written · your line kept'
        : `Notes written · all ${String(kept)} of your lines kept`,
  stoppedNothing: 'Stopped. Nothing was saved.',
  stoppedKept: (version: number) => `Stopped. Version ${String(version)} is unchanged.`,
} as const;

/** How each regenerate reason reads in the banner. */
export const reasonLabels: Readonly<Record<RegenerateReason, string>> = {
  eventChanged: 'the calendar event changed',
  attendeesChanged: 'attendees changed',
  transcriptEdited: 'the transcript was edited',
  notesEdited: 'you edited your notes',
  audioAppended: 'more of the meeting was transcribed',
};

/** "you edited your notes and the transcript was edited". */
export function reasonsText(reasons: readonly RegenerateReason[]): string {
  const labels = reasons.map((reason) => reasonLabels[reason]);
  if (labels.length <= 1) return labels.join('');
  return `${labels.slice(0, -1).join(', ')} and ${labels.at(-1) ?? ''}`;
}

/** `4:31`, or `1:04:31` from an hour on (as `deino-doc` writes citations). */
export function clockLabel(ms: number): string {
  const total = Math.floor(ms / 1000);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor(total / 60) % 60;
  const seconds = String(total % 60).padStart(2, '0');
  return hours > 0
    ? `${String(hours)}:${String(minutes).padStart(2, '0')}:${seconds}`
    : `${String(minutes)}:${seconds}`;
}
