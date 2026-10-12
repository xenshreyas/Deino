/**
 * The recipes feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
export type { Recipe, RecipeDraft, RecipeId, RecipeKind } from '@/lib/ipc/generated/recipes';

/** The Recipes page's two lists. */
export type RecipeTab = 'built-in' | 'mine';
