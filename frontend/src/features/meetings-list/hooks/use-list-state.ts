import { useState } from 'react';

import { usePatchSettings } from '@/features/settings';

import { filterKey } from '../filters';
import { usePageStore } from '../page-store';
import { useDeleteView, useSaveView } from '../queries';
import {
  ALL_MEETINGS,
  type MeetingFilter,
  type MeetingsListPrefs,
  type MeetingView,
  type MeetingViewId,
  type ViewChoice,
} from '../types';

/** The tab and filter on screen, and what changes them. */
export interface ListState {
  choice: ViewChoice;
  filter: MeetingFilter;
  /** The filter differs from the tab's own. */
  modified: boolean;
  setFilter: (filter: MeetingFilter) => void;
  choose: (choice: ViewChoice) => void;
  /** Saves the filter on screen as a new view (`id` null) or over `id`; with `kept`, only renames. */
  save: (name: string, id: MeetingViewId | null, kept?: MeetingFilter) => Promise<MeetingView>;
  remove: (id: MeetingViewId) => void;
}

/** The tab's own filter ("All meetings" while the views load). */
function baseFilter(choice: ViewChoice, views: readonly MeetingView[] | undefined): MeetingFilter {
  if (choice.kind === 'all') return ALL_MEETINGS;
  return views?.find((view) => view.id === choice.id)?.filter ?? ALL_MEETINGS;
}

/**
 * The page's tab and filter, starting from what the settings kept and
 * saving every change back (Circleback §14: "filters persist"). Mount it
 * once the settings have loaded; it owns the state from then on.
 */
export function useListState(
  initial: MeetingsListPrefs,
  views: readonly MeetingView[] | undefined,
): ListState {
  const [prefs, setPrefs] = useState(initial);
  const patch = usePatchSettings();
  const [saveView, deleteView] = [useSaveView(), useDeleteView()];
  const choice: ViewChoice =
    prefs.view === null ? { kind: 'all' } : { kind: 'saved', id: prefs.view };
  const persist = (next: MeetingsListPrefs) => {
    setPrefs(next);
    usePageStore.getState().setFilter(next.filter);
    patch.mutate({ ui: { meetingsList: next } });
  };
  const choose = (next: ViewChoice) => {
    persist({ view: next.kind === 'all' ? null : next.id, filter: baseFilter(next, views) });
  };
  return {
    choice,
    filter: prefs.filter,
    modified: filterKey(prefs.filter) !== filterKey(baseFilter(choice, views)),
    setFilter: (filter) => {
      persist({ ...prefs, filter });
    },
    choose,
    save: async (name, id, kept) => {
      const saved = await saveView.mutateAsync({
        id,
        view: { name, filter: kept ?? prefs.filter },
      });
      // The new view is not in `views` until they refetch: open it from the reply.
      // The new tab shows it was saved (UX-32: no toast).
      if (kept === undefined) persist({ view: saved.id, filter: saved.filter });
      return saved;
    },
    remove: (id) => {
      deleteView.mutate(id);
      if (choice.kind === 'saved' && choice.id === id) choose({ kind: 'all' });
    },
  };
}
