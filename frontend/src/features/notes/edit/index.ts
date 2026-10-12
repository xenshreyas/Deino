/**
 * Editing enhanced notes by asking (NOT-06), inside the notes feature: the
 * "Edit with AI" button and bar, and what the editor needs to scope an edit
 * to its selection.
 */
export { EditBar, type EditBarProps } from './edit-bar';
export { EDIT_SHORTCUT, EditWithAiButton, isEditShortcut } from './edit-button';
export { selectedTextblocks } from './selection';
export { closeEditBar, openEditBar, registerSelectionReader } from './store';
