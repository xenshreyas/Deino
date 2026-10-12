/**
 * The status row's "Edit with AI" button: opens the edit bar on the
 * editor's selection, or the whole notes. ⌘⇧E does the same while the
 * caret is in the notes (elsewhere on the page it stays Enhance).
 */
import { Button } from '@/ui/button';
import { ICON_PROPS, SquarePen } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { editCopy } from './copy';
import { openEditBar, useEditBarStore } from './store';

/** The shortcut, shown in the tooltip. */
export const EDIT_SHORTCUT = '⌘⇧E';

/** Props of {@link EditWithAiButton}. */
export interface EditWithAiButtonProps {
  disabled?: boolean;
}

/** Opens the edit bar. */
export function EditWithAiButton({ disabled = false }: EditWithAiButtonProps) {
  const open = useEditBarStore((state) => state.open);
  return (
    <Hint label={`${editCopy.buttonHint} · ${editCopy.shortcutHint}`} shortcut={EDIT_SHORTCUT}>
      <Button
        variant="ghost"
        size="sm"
        onClick={openEditBar}
        disabled={disabled}
        aria-pressed={open}
        // Keep the editor's selection: the bar reads it on click.
        onMouseDown={(event) => {
          event.preventDefault();
        }}
      >
        <SquarePen {...ICON_PROPS} />
        {editCopy.button}
      </Button>
    </Hint>
  );
}

/** True for ⌘⇧E (Ctrl⇧E off the Mac). */
export function isEditShortcut(event: KeyboardEvent): boolean {
  const mod = event.metaKey || event.ctrlKey;
  return mod && event.shiftKey && !event.altKey && event.key.toLowerCase() === 'e';
}
