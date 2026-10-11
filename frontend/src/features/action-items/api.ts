/**
 * Typed calls to the `action-items` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/action-items';
import { unwrap } from '@/lib/ipc/result';

import type {
  ActionItemEdit,
  ActionItemFilter,
  ActionItemId,
  ActionItemView,
  ActionItemViewId,
  FilterOptions,
  NewActionItem,
  SavedView,
  SavedViewInput,
  TodayDigest,
} from './types';

export const actionItemsApi = {
  /** The items `filter` matches, dated ones soonest first. */
  list: (filter: ActionItemFilter): Promise<ActionItemView[]> => unwrap(commands.list(filter)),
  /** The user's overdue and due-today open items. */
  today: (): Promise<TodayDigest> => unwrap(commands.today()),
  /** Adds an item the user wrote; rejects a blank title with `invalid`. */
  create: (item: NewActionItem): Promise<ActionItemView> => unwrap(commands.create(item)),
  /** Applies one edit to one item and returns its row. */
  update: (id: ActionItemId, edit: ActionItemEdit): Promise<ActionItemView> =>
    unwrap(commands.update(id, edit)),
  /** Applies a status, assignee, due or priority edit to many items; how many changed. */
  bulkUpdate: (ids: ActionItemId[], edit: ActionItemEdit): Promise<number> =>
    unwrap(commands.bulkUpdate(ids, edit)),
  /** Deletes the items; how many existed. */
  delete: (ids: ActionItemId[]): Promise<number> => unwrap(commands.delete(ids)),
  /** Completes every open item of the user's; how many. */
  markAllMineDone: (): Promise<number> => unwrap(commands.markAllMineDone()),
  /** People, companies, tags and folders to filter and assign by. */
  filterOptions: (): Promise<FilterOptions> => unwrap(commands.filterOptions()),
  /** The saved views, in tab order. */
  views: (): Promise<SavedView[]> => unwrap(commands.views()),
  /** Saves a new view; rejects a name in use with `conflict`. */
  createView: (view: SavedViewInput): Promise<SavedView> => unwrap(commands.createView(view)),
  /** Renames a view or replaces its filter. */
  updateView: (id: ActionItemViewId, view: SavedViewInput): Promise<SavedView> =>
    unwrap(commands.updateView(id, view)),
  /** Deletes a saved view. */
  deleteView: (id: ActionItemViewId): Promise<null> => unwrap(commands.deleteView(id)),
};
