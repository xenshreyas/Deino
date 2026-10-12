/** User-facing text of Settings → Shortcuts. */
export const hotkeysCopy = {
  section: {
    title: 'Hotkeys',
    description:
      'Hotkeys that work in every app, even when Deino is in the background. Deino’s own shortcuts (⌘K, ⌘N, ⌘J) work inside its window.',
  },
  descriptions: {
    record: 'Starts a recording, or stops the one running.',
    quick_note: 'Brings Deino forward on a new note.',
    ask: 'Opens Ask on the meeting you have open, or brings Deino forward.',
  },
  off: 'Off',
  change: 'Change',
  listening: 'Press keys…',
  listeningHint: 'Press the new combination, or Esc to cancel.',
  turnOff: 'Turn off',
  reset: 'Reset',
  active: 'On',
  loadFailed: 'Could not load the shortcuts.',
  links:
    'deino:// links open meetings, new notes and searches, and start or stop recording, from Raycast, Apple Shortcuts and scripts.',
} as const;
