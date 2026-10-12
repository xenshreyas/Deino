/**
 * The Recipes page's list: a quiet rail of names with their `/slug`, the
 * selected one in ink on a muted fill, multi-meeting built-ins marked WIP.
 */
import { cn } from '@/ui/cn';

import { recipesCopy } from '../copy';
import { isRunnable } from '../filter';
import type { Recipe, RecipeId } from '../types';

/** Props of {@link RecipeList}. */
export interface RecipeListProps {
  recipes: readonly Recipe[];
  selected: RecipeId | undefined;
  onSelect: (id: RecipeId) => void;
  empty: string;
}

/** One button per recipe. */
export function RecipeList({ recipes, selected, onSelect, empty }: RecipeListProps) {
  if (recipes.length === 0) return <p className="px-2 py-3 text-sm text-fg-muted">{empty}</p>;
  return (
    <ul className="grid gap-0.5" aria-label={recipesCopy.title}>
      {recipes.map((recipe) => (
        <li key={recipe.id}>
          <RecipeRow
            recipe={recipe}
            active={recipe.id === selected}
            onSelect={() => {
              onSelect(recipe.id);
            }}
          />
        </li>
      ))}
    </ul>
  );
}

function RecipeRow({
  recipe,
  active,
  onSelect,
}: {
  recipe: Recipe;
  active: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      aria-current={active ? 'true' : undefined}
      onClick={onSelect}
      className={cn(
        'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left',
        'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted',
        'focus-visible:outline-2 focus-visible:outline-focus-ring',
        active ? 'bg-bg-muted' : undefined,
      )}
    >
      <span className="min-w-0 flex-1">
        <span
          className={cn('block truncate text-sm', active ? 'font-semibold text-fg' : 'text-fg')}
        >
          {recipe.name}
        </span>
        <span className="block truncate font-mono text-xs text-fg-subtle">/{recipe.slug}</span>
      </span>
      {isRunnable(recipe) ? null : <WipBadge />}
    </button>
  );
}

/** The "WIP" badge of a recipe that needs several meetings (D13). */
export function WipBadge() {
  return (
    <span
      title={recipesCopy.wipBody}
      className="shrink-0 rounded-sm border border-border px-1 text-[10px] font-medium tracking-wide text-fg-subtle"
    >
      {recipesCopy.wip}
    </span>
  );
}
