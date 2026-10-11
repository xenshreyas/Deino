/** User-visible strings of the ⌘K palette. */
export const paletteCopy = {
  title: 'Command palette',
  placeholder: 'Search or jump to…',
  empty: 'No results',
  goTo: 'Go to',
  actions: 'Actions',
  meetings: 'Meetings',
  newNote: 'New note',
  newActionItem: 'New action item',
  startRecording: 'Start recording',
  askDeino: 'Ask Deino',
  /** Marks a feature that is not finished. */
  wip: 'WIP',
  importAudio: 'Import audio',
  toggleSidebar: 'Toggle sidebar',
  toggleRightPanel: 'Toggle right panel',
  theme: (name: string) => `Theme: ${name}`,
  hintOpen: 'Open',
  hintNavigate: 'Navigate',
  hintClose: 'Close',
} as const;
