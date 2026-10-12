/**
 * Which recipes a list or the `/` menu shows: pure functions over the
 * recipes, tested on their own.
 */
import type { Recipe, RecipeTab } from './types';

/** Whether a recipe can run on one meeting today (multi-meeting ones are WIP, D13). */
export function isRunnable(recipe: Recipe): boolean {
  return recipe.kind === 'single';
}

/** The recipes of `tab` whose name, slug, description or prompt contains `search`. */
export function filterRecipes(
  recipes: readonly Recipe[],
  tab: RecipeTab,
  search: string,
): Recipe[] {
  const needle = search.trim().toLowerCase();
  return recipes.filter((recipe) => {
    if (recipe.builtIn !== (tab === 'built-in')) return false;
    if (needle === '') return true;
    return [recipe.name, recipe.slug, recipe.description, recipe.prompt].some((field) =>
      field.toLowerCase().includes(needle),
    );
  });
}

/**
 * The runnable recipes for the `/` menu: slugs that start with `query` first,
 * then names or slugs that contain it.
 */
export function slashMatches(recipes: readonly Recipe[], query: string): Recipe[] {
  const needle = query.toLowerCase();
  const runnable = recipes.filter(isRunnable);
  const starts = runnable.filter((recipe) => recipe.slug.startsWith(needle));
  const contains = runnable.filter(
    (recipe) =>
      !recipe.slug.startsWith(needle) &&
      (recipe.slug.includes(needle) || recipe.name.toLowerCase().includes(needle)),
  );
  return [...starts, ...contains];
}
