import { toast } from 'sonner';

import { copy } from '../copy';
import { filterKey } from '../filters';
import { usePageStore } from '../page-store';
import { useDeleteView, useSaveView } from '../queries';
import {
  type ActionItemFilter,
  type ActionItemViewId,
  MY_OPEN_ITEMS,
  type SavedView,
  type ViewChoice,
} from '../types';

/** The filter on screen for a tab, with the edits made since it opened. */
export interface ViewFilter {
  filter: ActionItemFilter;
  /** The filter differs from the tab's own. */
  modified: boolean;
  setFilter: (filter: ActionItemFilter) => void;
  /**
   * Saves the filter on screen as a new view (`id` null) or as `id`, and
   * shows it; with `kept`, only renames view `id` (keeping filter `kept`).
   */
  save: (name: string, id: ActionItemViewId | null, kept?: ActionItemFilter) => Promise<SavedView>;
  remove: (id: ActionItemViewId) => void;
}

/** The tab's saved filter: "My items" or a saved view's (or "My items" while loading). */
export function baseFilter(choice: ViewChoice, views: readonly SavedView[] | undefined) {
  if (choice.kind === 'mine') return MY_OPEN_ITEMS;
  return views?.find((view) => view.id === choice.id)?.filter ?? MY_OPEN_ITEMS;
}

/**
 * The filter for `choice`: its saved filter until the user changes it in the
 * filter bar. Saving a view switches to it; deleting the open one goes back
 * to "My items".
 */
export function useViewFilter(
  choice: ViewChoice,
  views: readonly SavedView[] | undefined,
  onChoose: (choice: ViewChoice) => void,
): ViewFilter {
  const draft = usePageStore((state) => state.draft);
  const setDraft = usePageStore((state) => state.setDraft);
  const saveView = useSaveView();
  const deleteView = useDeleteView();
  const base = baseFilter(choice, views);
  const filter = draft ?? base;
  return {
    filter,
    modified: filterKey(filter) !== filterKey(base),
    setFilter: (next) => {
      setDraft(filterKey(next) === filterKey(base) ? null : next);
    },
    save: async (name, id, kept) => {
      const saved = await saveView.mutateAsync({ id, view: { name, filter: kept ?? filter } });
      if (kept === undefined) {
        setDraft(null);
        onChoose({ kind: 'saved', id: saved.id });
      }
      toast.success(copy.saved);
      return saved;
    },
    remove: (id) => {
      deleteView.mutate(id);
      if (choice.kind === 'saved' && choice.id === id) onChoose({ kind: 'mine' });
    },
  };
}
