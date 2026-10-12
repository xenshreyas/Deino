/** Every visible string of the vocabulary feature (TRN-05). */
export const vocabularyCopy = {
  title: 'Spelling',
  intro:
    'Names of the people and companies you have met are included automatically; add what Deino still mishears.',
  howItWorks:
    'Corrections are made while you record and again after each meeting. Every one can be undone from the meeting’s Details panel.',

  termsTitle: 'Words and names',
  termsDescription: 'Spelled exactly as you write them, along with what they can sound like.',
  termLabel: 'Word or name',
  termPlaceholder: 'Shreyas',
  soundsLikeLabel: 'Sounds like',
  soundsLikePlaceholder: 'Sreyas, Shrey as',
  soundsLike: (variants: readonly string[]) => `Sounds like ${variants.join(', ')}`,
  spellingOnly: 'Spelling only',
  termsEmpty: 'No words yet. Add a name or term Deino keeps getting wrong.',

  replacementsTitle: 'Replacements',
  replacementsDescription: 'Swap a phrase for another wherever it is heard as a whole word.',
  replaceLabel: 'Replace',
  replacePlaceholder: 'gonna',
  withLabel: 'With',
  withPlaceholder: 'going to',
  replacementsEmpty: 'No replacements yet.',

  add: 'Add',
  save: 'Save',
  cancel: 'Cancel',
  edit: (text: string) => `Edit ${text}`,
  remove: (text: string) => `Remove ${text}`,
  saveFailed: 'Could not save that entry',
  removeFailed: 'Could not remove that entry',
  loadError: 'Your vocabulary could not be loaded.',

  correctionsTitle: 'Vocabulary',
  correctionsCount: (count: number) =>
    count === 1 ? '1 line corrected' : `${count.toString()} lines corrected`,
  undo: 'Undo',
  undoAll: 'Undo all',
  undoLine: (from: string, to: string) => `Undo ${from} to ${to}`,
  edited: 'Edited since',
  undone: (restored: number) =>
    restored === 1 ? 'Correction undone' : `${restored.toString()} corrections undone`,
  undoFailed: 'Could not undo the correction',
} as const;
