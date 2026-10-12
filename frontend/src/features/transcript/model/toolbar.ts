/**
 * How many actions fit in a bubble's toolbar, and which go into its ⋯ menu.
 *
 * The toolbar sits beside the bubble on the side facing the middle, where a
 * bubble (at most 75% of the column) leaves a quarter of the column free.
 * Each action is a 24px button with a 2px gap; the time and the toolbar's
 * padding take the rest. On the record screen's narrow panel only two or
 * three fit, so the rarer actions (Edit, Clip, Comment) fold into ⋯ first.
 */
import type { TranscriptAction } from '../actions';

/** The share of the column a bubble leaves free beside it. */
const FREE_SHARE = 0.25;
/** The time label, the toolbar's padding and its gap from the bubble, in px. */
const TOOLBAR_CHROME_PX = 58;
/** One action button and its gap, in px. */
const SLOT_PX = 26;
/** Never fewer than this: an action and ⋯. */
const MIN_SLOTS = 2;

/** How many action buttons fit beside bubbles in a column `width` px wide. */
export function toolbarSlots(width: number): number {
  const slots = Math.floor((width * FREE_SHARE - TOOLBAR_CHROME_PX) / SLOT_PX);
  return Math.max(MIN_SLOTS, slots);
}

/** The actions shown on the toolbar, and those in its ⋯ menu. */
export interface ToolbarLayout {
  inline: TranscriptAction[];
  overflow: TranscriptAction[];
}

/**
 * Splits `shown` (in toolbar order) to fit `slots` buttons: all of them when
 * they fit; otherwise the common ones first, leaving a slot for ⋯, and the
 * rest (rare ones first to go) in the menu, both keeping toolbar order.
 */
export function layoutToolbar(shown: readonly TranscriptAction[], slots: number): ToolbarLayout {
  if (shown.length <= slots) return { inline: [...shown], overflow: [] };
  const room = Math.max(1, slots - 1);
  const byPriority = [
    ...shown.filter((one) => one.rare !== true),
    ...shown.filter((one) => one.rare === true),
  ];
  const kept = new Set(byPriority.slice(0, room));
  return {
    inline: shown.filter((one) => kept.has(one)),
    overflow: shown.filter((one) => !kept.has(one)),
  };
}
