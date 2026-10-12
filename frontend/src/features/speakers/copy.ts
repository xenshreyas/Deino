/** Every string the speakers feature shows, in one place (§3A). */
export const speakersCopy = {
  // Participants panel.
  panelLabel: 'Speakers',
  emptyBody: 'Once the recording is processed, each voice is listed here with how long it spoke.',
  loadError: 'Could not load the speakers.',
  retry: 'Try again',
  you: 'You',
  share: (percent: number) => `${percent}%`,
  talkTime: (name: string, duration: string, percent: number) =>
    `${name} spoke for ${duration}, ${percent}% of the meeting`,
  rowActions: (name: string) => `Actions for ${name}`,
  rename: 'Rename…',
  mergeInto: 'Merge into',
  thisIsMe: 'This is me',
  // Suggestion.
  suggestion: (name: string) => `Is this ${name}?`,
  yes: 'Yes',
  no: 'No',
  // Rename popover.
  assignTitle: (name: string) => `Who is ${name}?`,
  scopeLabel: 'Rename',
  scopeSegment: 'This segment',
  scopeAll: 'All segments',
  searchPlaceholder: 'Search or type a name',
  inMeeting: 'In this meeting',
  people: 'People',
  createNamed: (name: string) => `Add “${name}”`,
  noMatches: 'Type a name to add someone new.',
  meBadge: 'You',
  // Outcomes.
  remembered: (name: string) => `Deino will recognise ${name}’s voice next time.`,
  merged: 'Speakers merged',
  renameFailed: 'Could not rename the speaker',
  mergeFailed: 'Could not merge the speakers',
  answerFailed: 'Could not save your answer',
  // Regenerate notes.
  notesStale: 'Names changed. Regenerate the notes to use them.',
  regenerate: 'Regenerate notes',
  regenerating: 'Regenerating notes',
  regenerateFailed: 'Could not regenerate the notes',
  dismissNotice: 'Dismiss',
  // Settings → Recording → Speakers, and Advanced → Voices.
  settingsTitle: 'Speakers',
  settingsBody: 'Who said what, named on this Mac.',
  settingsIdentifyTitle: 'Recognise speakers by voice',
  settingsIdentifyBody:
    'After each meeting, Deino names the voices it knows and asks about the ones it is unsure of. Voices never leave this Mac.',
  profilesTitle: 'Voices',
  profilesBody:
    'Saved when you name a speaker for all of their segments. Each person keeps their five clearest samples.',
  profilesEmptyTitle: 'No voice profiles yet',
  profilesEmptyBody:
    'Open a meeting, choose a speaker in Participants and name them for all segments.',
  profilesLoadError: 'Could not load voice profiles.',
  samples: (count: number) => (count === 1 ? '1 sample' : `${count} samples`),
  fromMeeting: (title: string) => `From ${title}`,
  fromDeletedMeeting: 'From a deleted meeting',
  quality: (percent: number) => `${percent}% clear`,
  deleteSample: 'Delete sample',
  forget: (name: string) => `Forget ${name}’s voice`,
  forgetTitle: (name: string) => `Forget ${name}’s voice?`,
  forgetBody:
    'Deino will stop recognising them in new meetings. Names already given to speakers stay.',
  forgetConfirm: 'Forget voice',
  cancel: 'Cancel',
  deleteFailed: 'Could not delete the voice profile',
} as const;
