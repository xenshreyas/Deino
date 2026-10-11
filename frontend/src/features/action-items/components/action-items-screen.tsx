/**
 * The Action Items page (PARITY_PLAN AI-05; Circleback §8): the "Today"
 * digest, saved views as tabs, the filter bar (`F`), quick add, and every
 * item grouped by due date with inline edits, ticking and bulk actions.
 * The default tab is the user's own open items.
 */
import { Page, PageHeader } from '@/ui/page-header';

import { copy } from '../copy';
import { usePageShortcuts } from '../hooks/use-page-shortcuts';
import { useViewFilter, type ViewFilter } from '../hooks/use-view-filter';
import { usePageStore } from '../page-store';
import { useFilterOptions, useSavedViews } from '../queries';
import {
  type ActionItemFilter,
  type FilterOptions,
  MY_OPEN_ITEMS,
  type ViewChoice,
} from '../types';
import { ActionItemsList } from './action-items-list';
import { FilterBar } from './filter-bar';
import { PageMenu } from './page-menu';
import { QuickAdd } from './quick-add';
import { TodayDigest } from './today-digest';
import { ViewTabs } from './view-tabs';

/** Props of {@link ActionItemsScreen}; the route keeps the tab in the URL. */
export interface ActionItemsScreenProps {
  choice: ViewChoice;
  onChoose: (choice: ViewChoice) => void;
  /** Focus quick add (⌘K "New action item"). */
  adding?: boolean;
}

const NO_OPTIONS: FilterOptions = { people: [], companies: [], tags: [], folders: [] };

/** The filter bar, wired to the page store's menu state. */
function Filters({ view, options }: { view: ViewFilter; options: FilterOptions }) {
  const menuOpen = usePageStore((state) => state.menuOpen);
  const setMenuOpen = usePageStore((state) => state.setMenuOpen);
  return (
    <FilterBar
      filter={view.filter}
      options={options}
      onChange={view.setFilter}
      menuOpen={menuOpen}
      onMenuOpenChange={setMenuOpen}
    />
  );
}

/** The page. */
export function ActionItemsScreen({ choice, onChoose, adding = false }: ActionItemsScreenProps) {
  usePageShortcuts();
  const views = useSavedViews();
  const options = useFilterOptions().data ?? NO_OPTIONS;
  const reset = usePageStore((state) => state.reset);
  const choose = (next: ViewChoice) => {
    reset();
    onChoose(next);
  };
  const view = useViewFilter(choice, views.data, choose);
  const me = options.people.find((person) => person.isMe)?.id ?? null;
  const show = (filter: ActionItemFilter) => () => {
    view.setFilter(filter);
  };
  return (
    <Page>
      <PageHeader title={copy.title} actions={<PageMenu />} />
      <TodayDigest hideWhenEmpty onShowOverdue={show({ ...MY_OPEN_ITEMS, due: 'overdue' })} />
      <ViewTabs
        views={views.data ?? []}
        choice={choice}
        onChoose={choose}
        modified={view.modified}
        onSave={view.save}
        onDelete={view.remove}
      />
      <Filters view={view} options={options} />
      <QuickAdd assignee={me} autoFocus={adding} />
      <ActionItemsList
        filter={view.filter}
        people={options.people}
        onClearFilters={show(MY_OPEN_ITEMS)}
      />
    </Page>
  );
}
