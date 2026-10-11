/**
 * Bulk selection on the Action Items page (Circleback §8 "Bulk selection"):
 * click to toggle, Shift-click to select a range from the last click, ⌘A
 * for everything listed. Pure, over the list's IDs in display order.
 */
import type { ActionItemId } from './types';

/** The selected items and the anchor a Shift-click ranges from. */
export interface Selection {
  ids: ReadonlySet<ActionItemId>;
  anchor: ActionItemId | null;
}

/** Nothing selected. */
export const EMPTY_SELECTION: Selection = { ids: new Set(), anchor: null };

/** How a row was clicked. */
export interface SelectClick {
  id: ActionItemId;
  /** Shift held: select the range from the anchor. */
  range: boolean;
}

/** The selection after a click on a row's checkbox, given the list `order`. */
export function select(
  selection: Selection,
  click: SelectClick,
  order: readonly ActionItemId[],
): Selection {
  if (click.range && selection.anchor !== null) {
    const from = order.indexOf(selection.anchor);
    const to = order.indexOf(click.id);
    if (from !== -1 && to !== -1) {
      const span = order.slice(Math.min(from, to), Math.max(from, to) + 1);
      return { ids: new Set([...selection.ids, ...span]), anchor: click.id };
    }
  }
  const ids = new Set(selection.ids);
  if (ids.has(click.id)) ids.delete(click.id);
  else ids.add(click.id);
  return { ids, anchor: click.id };
}

/** Every listed item. */
export function selectAll(order: readonly ActionItemId[]): Selection {
  return { ids: new Set(order), anchor: order.at(-1) ?? null };
}

/** The selection limited to what is still listed (after a filter change or a delete). */
export function visible(selection: Selection, order: readonly ActionItemId[]): Selection {
  const listed = new Set(order);
  const ids = new Set([...selection.ids].filter((id) => listed.has(id)));
  if (ids.size === selection.ids.size) return selection;
  const anchor =
    selection.anchor !== null && listed.has(selection.anchor) ? selection.anchor : null;
  return { ids, anchor };
}
