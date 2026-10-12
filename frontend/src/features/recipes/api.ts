/**
 * Typed calls to the `recipes` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/recipes';
import { unwrap } from '@/lib/ipc/result';

import type { Recipe, RecipeDraft, RecipeId } from './types';

export const recipesApi = {
  /** Every recipe: the built-ins in library order, then the user's. */
  list: (): Promise<Recipe[]> => unwrap(commands.list()),
  /** Saves a new recipe; its `/slug` comes from its name. */
  create: (draft: RecipeDraft): Promise<Recipe> => unwrap(commands.create(draft)),
  /** Rewrites one of the user's recipes. */
  update: ({ id, draft }: { id: RecipeId; draft: RecipeDraft }): Promise<Recipe> =>
    unwrap(commands.update(id, draft)),
  /** Deletes one of the user's recipes. */
  remove: (id: RecipeId): Promise<null> => unwrap(commands.remove(id)),
  /** An editable copy of a recipe (Remix). */
  remix: (id: RecipeId): Promise<Recipe> => unwrap(commands.remix(id)),
  /** The recipes `ids` names (the user's own when empty) as a recipes file. */
  exportJson: (ids: RecipeId[]): Promise<string> => unwrap(commands.exportJson(ids)),
  /** Saves the recipes in a pasted file or `deino://recipe` share string. */
  importShared: (text: string): Promise<Recipe[]> => unwrap(commands.importShared(text)),
  /** The recipe's share string, `deino://recipe?data=…`. */
  shareLink: (id: RecipeId): Promise<string> => unwrap(commands.shareLink(id)),
};
