/** User-visible strings of editing notes by asking (NOT-06). */
export const editCopy = {
  button: 'Edit with AI',
  buttonHint: 'Edit the notes by asking',
  shortcutHint: 'In the notes',
  inputLabel: 'What should change?',
  placeholder: 'Make this more detailed, “Niall is spelled Nyal”, remove the pricing section…',
  wholeNotes: 'Whole notes',
  selection: (blocks: number) =>
    blocks === 1 ? 'Selected line' : `${String(blocks)} selected lines`,
  clearSelection: 'Edit the whole notes instead',
  submit: 'Propose edit',
  close: 'Close',
  drafting: 'Drafting the edit…',
  previewTitle: 'Proposed edit',
  changes: (count: number) => (count === 1 ? '1 change' : `${String(count)} changes`),
  renamed: (count: number) =>
    count === 1
      ? 'Renamed in 1 place, without AI'
      : `Renamed in ${String(count)} places, without AI`,
  layoutLabel: 'Diff layout',
  inline: 'Inline',
  split: 'Side by side',
  before: 'Now',
  after: 'After',
  added: 'Added',
  removed: 'Removed',
  accept: 'Accept',
  reject: 'Reject',
  accepted: 'Notes edited',
  undo: 'Undo',
  failed: 'The edit could not be made',
} as const;
