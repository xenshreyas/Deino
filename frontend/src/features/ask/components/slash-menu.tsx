/**
 * The `/` menu above the ask bar: the runnable recipes matching what the
 * user typed, the highlighted one in ink. The bar owns the keyboard (↑ ↓
 * Enter Tab Escape); this list is also clickable.
 */
import type { Recipe } from '@/features/recipes';
import { cn } from '@/ui/cn';

import { askCopy } from '../copy';

/** Props of {@link SlashMenu}. */
export interface SlashMenuProps {
  id: string;
  recipes: readonly Recipe[];
  active: number;
  onPick: (recipe: Recipe) => void;
}

/** The matching recipes. */
export function SlashMenu({ id, recipes, active, onPick }: SlashMenuProps) {
  return (
    <div className="absolute inset-x-3 bottom-full mb-1 overflow-hidden rounded-lg border border-border bg-bg shadow-md">
      {recipes.length === 0 ? (
        <p className="px-3 py-2 text-sm text-fg-muted">{askCopy.noRecipes}</p>
      ) : (
        <ul
          id={id}
          role="listbox"
          aria-label={askCopy.recipesLabel}
          className="max-h-64 overflow-y-auto py-1"
        >
          {recipes.map((recipe, index) => (
            <li
              key={recipe.id}
              id={`${id}-${recipe.slug}`}
              role="option"
              aria-selected={index === active}
              className={cn(
                'cursor-default px-3 py-1.5',
                index === active ? 'bg-bg-muted' : 'hover:bg-bg-subtle',
              )}
              onMouseDown={(event) => {
                event.preventDefault();
                onPick(recipe);
              }}
            >
              <span className="block truncate text-sm text-fg">{recipe.name}</span>
              <span className="block truncate font-mono text-xs text-fg-subtle">
                /{recipe.slug}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
