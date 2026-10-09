/**
 * The folder icon picker: a grid of the house icons (16px, 1.5 stroke) in
 * 32px cells; the current one sits on `--bg-muted` with ink.
 */
import { cn } from '@/ui/cn';
import { ICON_PROPS } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { foldersCopy } from '../copy';
import { FOLDER_ICONS } from '../folder-icons';

/** Props of {@link IconGrid}. */
export interface IconGridProps {
  /** The stored icon name; `null` is the default folder. */
  value: string | null;
  /** Called with the chosen name; the default folder clears the icon (`null`). */
  onChange: (icon: string | null) => void;
}

/** Fifteen icons in a five-column grid. */
export function IconGrid({ value, onChange }: IconGridProps) {
  const current = value ?? 'folder';
  return (
    <div role="group" aria-label={foldersCopy.iconTitle} className="grid grid-cols-5 gap-1">
      {FOLDER_ICONS.map((choice, index) => {
        const ChoiceIcon = choice.icon;
        const chosen = choice.name === current;
        return (
          <Hint key={choice.name} label={choice.label}>
            <button
              type="button"
              aria-label={choice.label}
              aria-pressed={chosen}
              onClick={() => {
                onChange(index === 0 ? null : choice.name);
              }}
              className={cn(
                'flex size-8 items-center justify-center rounded-md transition-colors duration-(--dur-fast)',
                chosen ? 'bg-bg-muted text-fg' : 'text-fg-muted hover:bg-bg-muted hover:text-fg',
              )}
            >
              <ChoiceIcon {...ICON_PROPS} />
            </button>
          </Hint>
        );
      })}
    </div>
  );
}
