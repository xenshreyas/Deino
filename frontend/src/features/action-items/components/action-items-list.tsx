/**
 * The page's list with its states (skeleton, inline error, empty) and the
 * bulk layer: the selection derived from what is listed, the floating bar
 * and the delete confirmation.
 */
import { Alert, AlertDescription } from '@/ui/alert';
import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { CircleAlert, ICON_PROPS, ListChecks } from '@/ui/icons';
import { Skeleton } from '@/ui/skeleton';

import { copy } from '../copy';
import { filterKey } from '../filters';
import { usePageStore } from '../page-store';
import {
  useActionItems,
  useBulkUpdate,
  useDeleteActionItems,
  useUpdateActionItem,
} from '../queries';
import { select, selectAll, type Selection, visible } from '../selection';
import {
  type ActionItemEdit,
  type ActionItemFilter,
  type ActionItemId,
  type ActionItemView,
  MY_OPEN_ITEMS,
  type PersonOption,
} from '../types';
import { BulkBar } from './bulk-bar';
import { DeleteDialog } from './delete-dialog';
import { ItemList } from './item-list';
import type { RowHandlers } from './item-row';

/** Props of {@link ActionItemsList}. */
export interface ActionItemsListProps {
  filter: ActionItemFilter;
  people: readonly PersonOption[];
  onClearFilters: () => void;
}

/** Row-shaped placeholders while the list loads. */
function ListSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-1 pt-2">
      {['w-3/5', 'w-2/5', 'w-2/3', 'w-1/3', 'w-1/2'].map((width) => (
        <div key={width} className="flex h-10 items-center gap-3 px-3">
          <Skeleton className="size-4 rounded-xs" />
          <Skeleton className={`h-2.5 ${width}`} />
        </div>
      ))}
    </div>
  );
}

/** Nothing to list: the user's own empty state, or the filters'. */
function Empty({
  filter,
  onClearFilters,
}: Pick<ActionItemsListProps, 'filter' | 'onClearFilters'>) {
  if (filterKey(filter) === filterKey(MY_OPEN_ITEMS)) {
    return (
      <EmptyState
        className="pt-[10vh]"
        icon={ListChecks}
        title={copy.empty.mineTitle}
        description={copy.empty.mineBody}
      />
    );
  }
  return (
    <EmptyState
      className="pt-[10vh]"
      icon={ListChecks}
      title={copy.empty.filteredTitle}
      description={copy.empty.filteredBody}
      action={
        <Button variant="secondary" onClick={onClearFilters}>
          {copy.filter.clear}
        </Button>
      }
    />
  );
}

/** The selection as the page shows it: ⌘A's "everything", or the clicked rows still listed. */
function useShownSelection(items: readonly ActionItemView[]): Selection {
  const selection = usePageStore((state) => state.selection);
  const allSelected = usePageStore((state) => state.allSelected);
  const order = items.map((item) => item.id);
  return allSelected ? selectAll(order) : visible(selection, order);
}

/** The bar for the selected `ids` and the delete confirmation. */
function BulkLayer({ ids, people }: { ids: ActionItemId[]; people: readonly PersonOption[] }) {
  const bulk = useBulkUpdate();
  const remove = useDeleteActionItems();
  const store = usePageStore();
  const apply = (edit: ActionItemEdit) => {
    bulk.mutate({ ids, edit });
  };
  return (
    <>
      <BulkBar
        count={ids.length}
        people={people}
        onEdit={apply}
        onDelete={() => {
          store.askDelete(ids);
        }}
        onClear={store.clearSelection}
      />
      <DeleteDialog
        count={store.deleting.length}
        onConfirm={() => {
          remove.mutate([...store.deleting]);
          store.clearSelection();
        }}
        onClose={() => {
          store.askDelete([]);
        }}
      />
    </>
  );
}

/** The listed items with bulk selection. */
function Loaded({ items, people }: { items: ActionItemView[]; people: readonly PersonOption[] }) {
  const update = useUpdateActionItem();
  const setSelection = usePageStore((state) => state.setSelection);
  const selection = useShownSelection(items);
  const order = items.map((item) => item.id);
  const handlers: RowHandlers = {
    onEdit: (item, edit) => {
      update.mutate({ id: item.id, edit });
    },
    onSelect: (click) => {
      setSelection(select(selection, click, order));
    },
  };
  return (
    <>
      <ItemList
        items={items}
        people={people}
        now={new Date()}
        selected={selection.ids}
        handlers={handlers}
      />
      <BulkLayer ids={[...selection.ids]} people={people} />
    </>
  );
}

/** Loading, error, empty or the list. */
export function ActionItemsList({ filter, people, onClearFilters }: ActionItemsListProps) {
  const items = useActionItems(filter);
  if (items.isPending) return <ListSkeleton />;
  if (items.isError) {
    return (
      <Alert variant="destructive">
        <CircleAlert {...ICON_PROPS} />
        <AlertDescription className="flex items-center justify-between gap-3">
          {copy.loadError}
          <Button variant="ghost" size="sm" onClick={() => void items.refetch()}>
            {copy.retry}
          </Button>
        </AlertDescription>
      </Alert>
    );
  }
  if (items.data.length === 0) return <Empty filter={filter} onClearFilters={onClearFilters} />;
  return <Loaded items={items.data} people={people} />;
}
