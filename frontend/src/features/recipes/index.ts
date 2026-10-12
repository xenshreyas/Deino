/**
 * Recipes (AI-04): saved prompts the user runs on a meeting. Built-in and
 * Mine, search, Remix, import, export and the `deino://recipe` share string.
 * Running one is the `ask` feature's: `/` in its bar, and the Preview the
 * `/recipes` route hands this page.
 *
 * | Export | Where |
 * |---|---|
 * | {@link RecipesScreen} | `/recipes` |
 * | {@link useRecipes}, {@link slashMatches} | the ask bar's `/` menu |
 *
 * Other code imports from here only.
 */
export { recipesApi } from './api';
export { type RecipePreviewComponent } from './components/recipe-detail';
export { RecipesScreen, type RecipesScreenProps } from './components/recipes-screen';
export { recipesCopy } from './copy';
export { filterRecipes, isRunnable, slashMatches } from './filter';
export { recipeKeys, useRecipes } from './queries';
export type { Recipe, RecipeDraft, RecipeId, RecipeKind, RecipeTab } from './types';
