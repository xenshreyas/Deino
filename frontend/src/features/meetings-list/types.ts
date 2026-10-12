/**
 * The Meetings page's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { MeetingFilter, MeetingViewId } from '@/lib/ipc/generated/meetings';

export type { FilterOptions } from '@/lib/ipc/generated/action-items';
export type {
  Audience,
  ListedAttendee,
  ListedMeeting,
  ListedTag,
  MeetingCursor,
  MeetingFilter,
  MeetingId,
  MeetingPage,
  MeetingView,
  MeetingViewId,
  MeetingViewInput,
  MergeOutcome,
  TagId,
  TrashedMeeting,
  TrashOutcome,
} from '@/lib/ipc/generated/meetings';
export type { MeetingsListPrefs } from '@/lib/ipc/generated/settings';

/** The filter of "All meetings": nothing narrows the list. */
export const ALL_MEETINGS: MeetingFilter = {
  dateRange: null,
  people: [],
  companies: [],
  tags: [],
  folders: [],
  hasActionItems: false,
  audience: null,
};

/** The open tab: "All meetings" or a saved view. */
export type ViewChoice = { kind: 'all' } | { kind: 'saved'; id: MeetingViewId };

/**
 * Brands a saved view's ID that arrived as text (the settings document).
 * IDs are minted by Rust, so the only check is that there is one.
 */
export function asMeetingViewId(raw: string): MeetingViewId {
  if (raw.trim() === '') throw new TypeError('A view ID cannot be empty.');
  return raw as MeetingViewId;
}
