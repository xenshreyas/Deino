/**
 * TanStack Query hooks for recipes. The list names the `recipes` table, so
 * `DbChanged` refreshes it after any write.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { recipesApi } from './api';

/** The tables recipes read. */
const RECIPE_TABLES = ['recipes'] as const satisfies readonly Table[];

/** Query keys; every key starts with `recipeKeys.all`. */
export const recipeKeys = {
  all: ['recipes'] as const,
  list: () => [...recipeKeys.all, 'list'] as const,
};

/** Every recipe. */
export function useRecipes() {
  return useQuery({
    queryKey: recipeKeys.list(),
    queryFn: recipesApi.list,
    meta: { tables: RECIPE_TABLES },
  });
}

/** Refetches the list once a mutation settles. */
function useInvalidate() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: recipeKeys.all });
}

/** Saves a new recipe. */
export function useCreateRecipe() {
  return useMutation({ mutationFn: recipesApi.create, onSettled: useInvalidate() });
}

/** Rewrites one of the user's recipes. */
export function useUpdateRecipe() {
  return useMutation({ mutationFn: recipesApi.update, onSettled: useInvalidate() });
}

/** Deletes one of the user's recipes. */
export function useRemoveRecipe() {
  return useMutation({ mutationFn: recipesApi.remove, onSettled: useInvalidate() });
}

/** Makes an editable copy of a recipe. */
export function useRemixRecipe() {
  return useMutation({ mutationFn: recipesApi.remix, onSettled: useInvalidate() });
}

/** Imports recipes from a file or a share string. */
export function useImportRecipes() {
  return useMutation({ mutationFn: recipesApi.importShared, onSettled: useInvalidate() });
}
