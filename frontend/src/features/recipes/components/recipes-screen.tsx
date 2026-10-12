/**
 * The Recipes page (AI-04, Granola §5.3): Built-in and Mine, a search over
 * names and prompts, the selected recipe on the right with its prompt,
 * Remix or the editor, sharing, and a Preview on a chosen meeting. "New
 * recipe" opens an empty editor; Import reads a file or a share string.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Input } from '@/ui/input';
import { Download, ICON_PROPS, Plus, Upload } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';
import { SegmentedControl } from '@/ui/segmented-control';
import { Skeleton } from '@/ui/skeleton';

import { recipesApi } from '../api';
import { recipesCopy } from '../copy';
import { filterRecipes } from '../filter';
import { useCreateRecipe, useRecipes } from '../queries';
import type { Recipe, RecipeId, RecipeTab } from '../types';
import { ImportDialog } from './import-dialog';
import { RecipeDetail, type RecipePreviewComponent } from './recipe-detail';
import { RecipeForm } from './recipe-form';
import { RecipeList } from './recipe-list';

/** Props of {@link RecipesScreen}. */
export interface RecipesScreenProps {
  /** Previews a recipe on a meeting (the `ask` feature's, given by the route). */
  Preview: RecipePreviewComponent;
}

const TABS = [
  { value: 'built-in', label: recipesCopy.builtIn },
  { value: 'mine', label: recipesCopy.mine },
] as const;

/** The page's state: the list, the search and the selection. */
function useRecipesPage() {
  const recipes = useRecipes();
  const [tab, setTab] = useState<RecipeTab>('built-in');
  const [search, setSearch] = useState('');
  const [selected, setSelected] = useState<RecipeId | 'new' | undefined>(undefined);
  const all = recipes.data ?? [];
  const shown = filterRecipes(all, tab, search);
  const select = (id: RecipeId | undefined) => {
    const recipe = all.find((candidate) => candidate.id === id);
    // An ID not listed yet is a recipe just saved, remixed or imported: the user's.
    if (id !== undefined && recipe?.builtIn !== true) setTab('mine');
    setSelected(id);
  };
  const current =
    selected === 'new'
      ? ('new' as const)
      : (all.find((recipe) => recipe.id === selected) ?? shown[0]);
  const startNew = () => {
    setSelected('new');
  };
  const loading = recipes.isPending;
  return { loading, all, shown, tab, setTab, search, setSearch, select, current, startNew };
}

/** What the rail and the detail read. */
type PageState = ReturnType<typeof useRecipesPage>;

/** The page. */
export function RecipesScreen({ Preview }: RecipesScreenProps) {
  const page = useRecipesPage();
  const [importing, setImporting] = useState(false);
  const actions = (
    <Actions
      all={page.all}
      onImport={() => {
        setImporting(true);
      }}
      onNew={page.startNew}
    />
  );
  return (
    <Page className="max-w-[1100px]">
      <PageHeader
        title={recipesCopy.title}
        description={recipesCopy.description}
        actions={actions}
      />
      <div className="grid grid-cols-1 gap-8 md:grid-cols-[16rem_minmax(0,1fr)]">
        <Rail page={page} />
        <Detail current={page.current} Preview={Preview} onSelect={page.select} />
      </div>
      <ImportDialog
        open={importing}
        onClose={() => {
          setImporting(false);
        }}
        onImported={(saved) => {
          setImporting(false);
          page.select(saved[0]?.id);
        }}
      />
    </Page>
  );
}

/** The tabs, the search and the list. */
function Rail({ page }: { page: PageState }) {
  const empty =
    page.search === '' && page.tab === 'mine' ? recipesCopy.emptyMine : recipesCopy.emptySearch;
  return (
    <div className="grid content-start gap-3">
      <SegmentedControl
        label={recipesCopy.tabsLabel}
        segments={TABS}
        value={page.tab}
        onValueChange={page.setTab}
      />
      <Input
        type="search"
        aria-label={recipesCopy.search}
        placeholder={recipesCopy.search}
        value={page.search}
        onChange={(event) => {
          page.setSearch(event.target.value);
        }}
      />
      {page.loading ? (
        <ListSkeleton />
      ) : (
        <RecipeList
          recipes={page.shown}
          selected={page.current === 'new' ? undefined : page.current?.id}
          onSelect={page.select}
          empty={empty}
        />
      )}
    </div>
  );
}

/** Export mine, Import and New recipe. */
function Actions({
  all,
  onImport,
  onNew,
}: {
  all: readonly Recipe[];
  onImport: () => void;
  onNew: () => void;
}) {
  const exportMine = async () => {
    if (!all.some((recipe) => !recipe.builtIn)) {
      toast(recipesCopy.nothingToExport);
      return;
    }
    await navigator.clipboard.writeText(await recipesApi.exportJson([]));
    toast.success(recipesCopy.exported);
  };
  return (
    <>
      <Button variant="ghost" onClick={() => void exportMine()}>
        <Download {...ICON_PROPS} />
        {recipesCopy.exportMine}
      </Button>
      <Button variant="secondary" onClick={onImport}>
        <Upload {...ICON_PROPS} />
        {recipesCopy.import}
      </Button>
      <Button onClick={onNew}>
        <Plus {...ICON_PROPS} />
        {recipesCopy.newRecipe}
      </Button>
    </>
  );
}

/** The selected recipe, the new-recipe editor, or a hint. */
function Detail({
  current,
  Preview,
  onSelect,
}: {
  current: Recipe | 'new' | undefined;
  Preview: RecipePreviewComponent;
  onSelect: (id: RecipeId | undefined) => void;
}) {
  if (current === 'new') return <NewRecipe onCreated={onSelect} />;
  if (current === undefined) return <p className="text-sm text-fg-muted">{recipesCopy.pick}</p>;
  return <RecipeDetail key={current.id} recipe={current} Preview={Preview} onSelect={onSelect} />;
}

function NewRecipe({ onCreated }: { onCreated: (id: RecipeId) => void }) {
  const create = useCreateRecipe();
  return (
    <section className="grid content-start gap-4" aria-label={recipesCopy.newRecipe}>
      <h2 className="text-lg font-semibold text-fg">{recipesCopy.newRecipe}</h2>
      <RecipeForm
        initial={{ name: '', description: '', prompt: '' }}
        saving={create.isPending}
        onSave={(draft) => {
          create.mutate(draft, {
            onSuccess: (recipe) => {
              onCreated(recipe.id);
            },
          });
        }}
      />
    </section>
  );
}

function ListSkeleton() {
  return (
    <div className="grid gap-2">
      <Skeleton className="h-8 w-full" />
      <Skeleton className="h-8 w-full" />
      <Skeleton className="h-8 w-3/4" />
    </div>
  );
}
