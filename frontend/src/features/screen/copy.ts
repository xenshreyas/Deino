/** Every visible string of the screen feature. */
export const screenCopy = {
  section: {
    title: 'Meeting window',
    description:
      'While Deino records a detected meeting, it can keep snapshots of what is shared in the meeting window and a video of it. Everything stays on this Mac.',
  },
  permission: {
    title: 'Permission',
    name: 'Screen Recording',
    granted: 'Allowed',
    missing: 'Not allowed',
    unsupported: 'Not available on this computer.',
    why: 'macOS asks for it before any app can see a window.',
    hint: 'Allow Deino under Privacy & Security → Screen & System Audio Recording. Deino never asks on its own during a meeting; this page updates when you come back.',
    openSettings: 'Open System Settings',
    settingsFailed:
      'Could not open System Settings. Open Privacy & Security → Screen & System Audio Recording there.',
  },
  capture: {
    title: 'Capture',
    detected: 'Capture detected meetings',
    detectedOn: 'The meeting window of Zoom, Teams, Meet and other detected calls.',
    detectedOff: 'Off: nothing on screen is captured.',
    consent: 'Always get consent before capturing a shared screen.',
    snapshots: 'Keep snapshots',
    snapshotsHint: 'A picture when the shared screen changes, at most every 10 seconds.',
    video: 'Record video',
    videoHint: 'The meeting window beside the audio, with its sound, for playback and clips.',
    frameRate: 'Frame rate',
    rates: { 1: '1 fps', 2: '2 fps', 5: '5 fps' },
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
    never: "Don't show again",
  },
  picker: {
    label: 'Screen to capture',
    none: 'Nothing to capture: allow Screen Recording in Settings.',
    failed: 'Could not capture that window.',
  },
  saveFailed: 'Could not save the screen capture setting.',
  loadFailed: 'Could not read the screen capture status.',
} as const;
