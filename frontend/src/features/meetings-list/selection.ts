/**
 * Bulk selection on the Meetings page (Circleback §14 "Bulk actions"):
 * click to toggle, Shift-click to select a range from the last click, ⌘A
 * for every meeting the filter matches. Pure, over the listed IDs in
 * display order.
 */
import type { MeetingId } from './types';

/** The selected meetings and the anchor a Shift-click ranges from. */
export interface Selection {
  ids: ReadonlySet<MeetingId>;
  anchor: MeetingId | null;
}

/** Nothing selected. */
export const EMPTY_SELECTION: Selection = { ids: new Set(), anchor: null };

/** How a row was clicked. */
export interface SelectClick {
  id: MeetingId;
  /** Shift held: select the range from the anchor. */
  range: boolean;
}

/** The selection after a click on a row's checkbox, given the list `order`. */
export function select(
  selection: Selection,
  click: SelectClick,
  order: readonly MeetingId[],
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

/** Exactly `ids` (⌘A: every match, loaded or not). */
export function selectAll(ids: readonly MeetingId[]): Selection {
  return { ids: new Set(ids), anchor: ids.at(-1) ?? null };
}
