/**
 * The small toolbar above a text selection (ui.md floating layer): one
 * button per action, on the raised surface with a soft shadow. Buttons keep
 * the selection (mouse down does not move it); running one clears it.
 */
import { createPortal } from 'react-dom';

import { cn } from '@/ui/cn';
import { floatingSurface } from '@/ui/popover';
import { ICON_PROPS, type Icon } from '@/ui/icons';

import { commentsCopy } from '../copy';
import type { ScreenRect } from '../types';

/** One button of the menu. */
export interface SelectionAction {
  id: string;
  label: string;
  icon: Icon;
  run: () => void;
}

/** Props of {@link SelectionMenu}. */
export interface SelectionMenuProps {
  rect: ScreenRect;
  actions: readonly SelectionAction[];
}

/** How far above the selection the menu sits. */
const GAP_PX = 6;

/** The toolbar, fixed above the selection's middle. */
export function SelectionMenu({ rect, actions }: SelectionMenuProps) {
  if (actions.length === 0) return null;
  return createPortal(
    <div
      role="toolbar"
      aria-label={commentsCopy.selectionMenu}
      style={{ left: rect.left + rect.width / 2, top: rect.top - GAP_PX }}
      className={cn(
        floatingSurface,
        'fixed z-(--z-popover) flex -translate-x-1/2 -translate-y-full items-center gap-0.5 rounded-md p-0.5',
      )}
    >
      {actions.map((action) => {
        const ActionIcon = action.icon;
        return (
          <button
            key={action.id}
            type="button"
            onMouseDown={(event) => {
              event.preventDefault();
            }}
            onClick={() => {
              action.run();
              window.getSelection()?.removeAllRanges();
            }}
            className="flex h-7 items-center gap-1.5 rounded-sm px-2 text-sm text-fg-muted transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring"
          >
            <ActionIcon {...ICON_PROPS} size={14} />
            {action.label}
          </button>
        );
      })}
    </div>,
    document.body,
  );
}
