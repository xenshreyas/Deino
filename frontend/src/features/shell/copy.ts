/** User-visible strings of the app shell. */
export const shellCopy = {
  sidebarLabel: 'Sidebar',
  toggleSidebar: 'Toggle sidebar',
  resizeSidebar: 'Resize sidebar',
  newNote: 'New note',
  search: 'Search',
  importAudio: 'Import recording',
  startRecording: 'Start recording',
  openLiveMeeting: 'Open the live meeting',
  paused: 'Paused',
  saving: 'Saving…',
  starting: 'Starting…',
  stopRecording: 'Stop recording',
  untitledRecording: 'New recording',

  comingSoonTitle: (feature: string) => `${feature} is on its way`,
  comingSoonAction: 'Back to Home',

  chatLabel: 'Ask Deino',
  chatPlaceholder: 'Ask anything',
  chatComingSoon: 'Chat across all your meetings is coming soon.',
  chatSoonToast: 'Ask Deino is coming soon.',
  close: 'Close',
} as const;
