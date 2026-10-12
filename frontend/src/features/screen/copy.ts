/** Every visible string of the screen feature. */
export const screenCopy = {
  section: {
    title: 'Screen capture',
    description:
      'While Deino records a detected meeting, it can keep snapshots of what is shared in the meeting window and a video of it. Everything stays on this Mac.',
  },
  /** The permission, as the Permissions card and its link name it. */
  permission: 'Screen Recording',
  capture: {
    title: 'Capture',
    more: 'Snapshots and video',
    detected: 'Capture detected meetings',
    detectedOn: 'The meeting window of Zoom, Teams, Meet and other detected calls.',
    detectedOff: 'Off: nothing on screen is captured.',
    consent: 'Always get consent before capturing a shared screen.',
    snapshots: 'Keep snapshots',
    snapshotsHint: 'A picture when the shared screen changes, at most every 10 seconds.',
    video: 'Record video',
    videoHint: 'The meeting window beside the audio, with its sound, for playback and clips.',
    frameRate: 'Video smoothness',
    rates: { 1: 'Once a second', 2: 'Twice a second', 5: '5 times a second' },
    describe: 'Describe snapshots with Claude',
    describeHint:
      'Sends each snapshot to Claude after the meeting so the notes can say what a chart or design showed. Off: only text read on this Mac reaches the notes.',
  },
  images: {
    title: 'Images',
    open: (time: string) => `Snapshot at ${time}`,
    delete: 'Delete snapshot',
    deleteFailed: 'Could not delete the snapshot.',
  },
  reminder: {
    label: 'Screen capture reminder',
    title: 'Deino is capturing the shared screen',
    body: 'Let everyone know their shared screen is being saved.',
    dismiss: 'Got it',
    never: 'Don’t show again',
  },
  picker: {
    label: 'Screen to capture',
    none: 'Nothing to capture: allow Screen Recording in Settings.',
    failed: 'Could not capture that window.',
  },
  saveFailed: 'Couldn’t save the screen capture setting.',
  loadFailed: 'Couldn’t read the screen capture status.',
} as const;
