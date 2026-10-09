/** Every string the transcription feature shows, in one place (§3A). */
export const copy = {
  title: 'Parakeet Unified EN 0.6B',
  description:
    'Live, private transcription on this Mac. Pick a size: larger files are slightly more accurate and use more memory.',
  loading: 'Loading models…',
  loadFailed: 'The model list could not be loaded.',
  recommended: 'Recommended for this Mac',
  ready: 'Downloaded',
  missing: 'Not downloaded',
  inUse: 'In use',
  use: 'Use',
  download: 'Download',
  resume: 'Resume',
  cancel: 'Cancel',
  delete: 'Delete',
  downloading: (percent: number) => `Downloading… ${String(percent)}%`,
  paused: (percent: number) => `Paused at ${String(percent)}%`,
  size: (megabytes: number) => `${String(megabytes)} MB`,
  live: 'Live transcript',
  licence: 'Licensed by NVIDIA Corporation under the NVIDIA Open Model License.',
  licenceLink: 'Read the licence',
  noticeTitle: 'Notices',
  noticeModel:
    'Speech recognition uses Parakeet Unified EN 0.6B (GGUF conversion by handy-computer of nvidia/parakeet-unified-en-0.6b).',
} as const;
