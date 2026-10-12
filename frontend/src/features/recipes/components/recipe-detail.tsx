/**
 * The selected recipe: a built-in's prompt read-only with Remix, or the
 * user's own in the editor with Delete; both can be shared as a link or
 * JSON, and a single-meeting recipe can be previewed on a meeting.
 */
import type { ComponentType } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Copy, ICON_PROPS, Link, Sparkles, Trash2 } from '@/ui/icons';

import { recipesApi } from '../api';
import { recipesCopy } from '../copy';
import { isRunnable } from '../filter';
import { useRemixRecipe, useRemoveRecipe, useUpdateRecipe } from '../queries';
import type { Recipe, RecipeId } from '../types';
import { RecipeForm } from './recipe-form';
import { WipBadge } from './recipe-list';

/** What previews a recipe on a meeting (the `ask` feature's, given by the route). */
export type RecipePreviewComponent = ComponentType<{ recipe: Recipe }>;

/** Props of {@link RecipeDetail}. */
export interface RecipeDetailProps {
  recipe: Recipe;
  Preview: RecipePreviewComponent;
  /** Selects another recipe (the remix, or none after a delete). */
  onSelect: (id: RecipeId | undefined) => void;
}

/** The recipe's header, body, sharing and preview. */
export function RecipeDetail({ recipe, Preview, onSelect }: RecipeDetailProps) {
  return (
    <section className="grid min-w-0 content-start gap-6" aria-label={recipe.name}>
      <header className="grid gap-1">
        <div className="flex items-center gap-2">
          <h2 className="truncate text-lg font-semibold text-fg">{recipe.name}</h2>
          {isRunnable(recipe) ? null : <WipBadge />}
        </div>
        <p className="font-mono text-xs text-fg-subtle">/{recipe.slug}</p>
        {recipe.description === '' ? null : (
          <p className="text-sm text-pretty text-fg-muted">{recipe.description}</p>
        )}
      </header>
      {recipe.builtIn ? (
        <BuiltIn recipe={recipe} onSelect={onSelect} />
      ) : (
        <Mine recipe={recipe} onSelect={onSelect} />
      )}
      {isRunnable(recipe) ? <Preview recipe={recipe} /> : null}
    </section>
  );
}

/** A built-in: the prompt, Remix and sharing. */
function BuiltIn({ recipe, onSelect }: Pick<RecipeDetailProps, 'recipe' | 'onSelect'>) {
  const remix = useRemixRecipe();
  return (
    <div className="grid gap-3">
      <p className="rounded-lg bg-bg-subtle px-3 py-2.5 text-sm whitespace-pre-wrap text-fg">
        {recipe.prompt}
      </p>
      <div className="flex flex-wrap items-center gap-2">
        {isRunnable(recipe) ? (
          <Button
            variant="secondary"
            disabled={remix.isPending}
            onClick={() => {
              remix.mutate(recipe.id, {
                onSuccess: (copy) => {
                  toast.success(recipesCopy.remixed);
                  onSelect(copy.id);
                },
              });
            }}
          >
            <Sparkles {...ICON_PROPS} />
            {recipesCopy.remix}
          </Button>
        ) : null}
        <ShareButtons recipe={recipe} />
      </div>
    </div>
  );
}

/** One of the user's recipes: the editor, sharing and Delete. */
function Mine({ recipe, onSelect }: Pick<RecipeDetailProps, 'recipe' | 'onSelect'>) {
  const update = useUpdateRecipe();
  const remove = useRemoveRecipe();
  const deleteButton = (
    <Button
      variant="danger"
      disabled={remove.isPending}
      onClick={() => {
        remove.mutate(recipe.id, {
          onSuccess: () => {
            toast.success(recipesCopy.deleted);
            onSelect(undefined);
          },
        });
      }}
    >
      <Trash2 {...ICON_PROPS} />
      {recipesCopy.delete}
    </Button>
  );
  return (
    <RecipeForm
      initial={{ name: recipe.name, description: recipe.description, prompt: recipe.prompt }}
      saving={update.isPending}
      onSave={(draft) => {
        update.mutate(
          { id: recipe.id, draft },
          { onSuccess: () => toast.success(recipesCopy.saved) },
        );
      }}
      actions={
        <>
          <ShareButtons recipe={recipe} />
          <span className="flex-1" />
          {deleteButton}
        </>
      }
    />
  );
}

/** Copies the share string or the recipe as JSON. */
function ShareButtons({ recipe }: { recipe: Recipe }) {
  const copy = async (text: Promise<string>, done: string) => {
    await navigator.clipboard.writeText(await text);
    toast.success(done);
  };
  return (
    <>
      <Button
        variant="ghost"
        onClick={() => void copy(recipesApi.shareLink(recipe.id), recipesCopy.linkCopied)}
      >
        <Link {...ICON_PROPS} />
        {recipesCopy.copyLink}
      </Button>
      <Button
        variant="ghost"
        onClick={() => void copy(recipesApi.exportJson([recipe.id]), recipesCopy.jsonCopied)}
      >
        <Copy {...ICON_PROPS} />
        {recipesCopy.copyJson}
      </Button>
    </>
  );
}
