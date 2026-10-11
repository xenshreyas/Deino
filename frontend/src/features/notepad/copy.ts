/** User-visible strings of the notepad. */
export const notepadCopy = {
  placeholder: 'Write anything. Deino will fill in the rest.',
  editorLabel: 'My notes',
  saving: 'Saving…',
  saved: 'Saved',
  notSaved: 'Not saved yet, retrying',
  loadError: 'These notes could not be opened.',
  retry: 'Try again',
  mentionsLabel: 'People',
  noPeople: 'No one by that name yet',
  addPerson: (name: string) => `Add “${name}”`,
  imageFailedTitle: 'The image was not added',
  templateFailedTitle: 'The template was not set',
  viewLabel: 'Notes view',
  myNotes: 'My notes',
  liveHeading: 'Notes',
  transcriptHeading: 'Transcript',
} as const;
