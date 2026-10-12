/** Every visible string of the import feature. */
import type { ImportFailure } from './types';

export const importCopy = {
  dialogTitle: 'Import a recording',
  dialogDescription:
    'Deino converts the audio, keeps any video for playback, and transcribes it on this Mac.',
  chooseFile: 'Choose file…',
  chooseHint: 'Or drop an audio or video file anywhere in the window.',
  checking: 'Checking the file…',
  titleLabel: 'Title',
  titlePlaceholder: 'The file’s name',
  video: 'Video',
  audio: 'Audio',
  import: 'Import',
  cancel: 'Cancel',
  importing: 'Converting the audio…',
  importingHint: 'A long recording takes a little while. You can keep using Deino.',
  importedTitle: 'Imported',
  importedHint: 'Deino is transcribing it now. Notes follow as usual.',
  linkHint: 'Link it to the calendar event it was recorded for:',
  openMeeting: 'Open meeting',
  done: 'Done',
  chooseAnother: 'Choose another file',
  minutes: (seconds: number) =>
    seconds < 60 ? `${String(seconds)} s` : `${String(Math.round(seconds / 60))} min`,

  // Settings → Watched folders
  intro:
    'Each new audio or video file is imported once it has stopped changing for 10 seconds, and never twice. Files already in a folder when you add it are left alone.',
  foldersTitle: 'Folders',
  foldersDescription: 'Deino checks these folders every few seconds while it is open.',
  addFolder: 'Add folder…',
  noFoldersTitle: 'No watched folders',
  noFoldersHint: 'Add the folder your recorder or screen recordings save into.',
  moveLabel: 'Move to processed',
  moveHint: 'Moves each imported file into a Processed folder inside it.',
  remove: 'Remove',
  removeLabel: (path: string) => `Stop watching ${path}`,
  failuresTitle: 'Couldn’t import',
  failuresDescription: 'Files that were skipped. Retry once the file is fixed, or dismiss it.',
  noFailures: 'Nothing has failed.',
  retry: 'Retry',
  dismiss: 'Dismiss',
  loadError: 'Watched folders could not be loaded.',
  addFailed: 'Couldn’t add the folder',
  changeFailed: 'Couldn’t save the change',
  failure: {
    missing: 'The file was moved or deleted before it could be read.',
    too_large: 'The file is larger than 20 GB.',
    no_audio: 'The file has no audio.',
    unsupported: 'Deino can’t read this format.',
    damaged: 'The audio is damaged.',
    failed: 'The meeting couldn’t be saved.',
  } satisfies Record<ImportFailure, string>,

  // The window drop
  dropTitle: 'Drop to import',
  unsupportedDropTitle: 'This file can’t be imported',
  supportedFormats: (list: string) => `Deino imports ${list}.`,
} as const;
