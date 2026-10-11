/**
 * The action items feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type {
  ActionItemFilter,
  ActionItemId,
  ActionItemView,
  ActionItemViewId,
} from '@/lib/ipc/generated/action-items';

export type {
  ActionItemEdit,
  ActionItemFilter,
  ActionItemId,
  ActionItemPriority,
  ActionItemStatus,
  ActionItemView,
  ActionItemViewId,
  AssigneeScope,
  CompanyOption,
  DateRange,
  DueFilter,
  FilterOptions,
  FolderOption,
  NewActionItem,
  PersonId,
  PersonOption,
  SavedView,
  SavedViewInput,
  StatusFilter,
  TagOption,
  TodayDigest,
} from '@/lib/ipc/generated/action-items';

/** The user's own open items: the page's first tab and default filter. */
export const MY_OPEN_ITEMS: ActionItemFilter = {
  assignee: { kind: 'me' },
  status: 'open',
  meetingId: null,
  tagId: null,
  companyId: null,
  folderId: null,
  due: null,
  dateRange: null,
};

/** A view to show: the built-in "My items" tab or a saved view. */
export type ViewChoice = { kind: 'mine' } | { kind: 'saved'; id: ActionItemViewId };

/** Brands an item ID that arrived as a plain string (a URL, a DOM attribute). */
export function asActionItemId(raw: string): ActionItemId {
  if (raw.trim() === '') throw new TypeError('An action item ID cannot be empty.');
  return raw as ActionItemId;
}

/** Brands a saved view's ID that arrived as a plain string (a URL parameter). */
export function asViewId(raw: string): ActionItemViewId {
  if (raw.trim() === '') throw new TypeError('A view ID cannot be empty.');
  return raw as ActionItemViewId;
}

/** True when the item is done. */
export function isDone(item: Pick<ActionItemView, 'status'>): boolean {
  return item.status === 'done';
}
