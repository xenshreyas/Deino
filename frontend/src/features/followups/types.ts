/**
 * The follow-ups feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { MeetingId } from '@/lib/ipc/generated/meetings';

export type {
  FollowUpDraft,
  FollowUpEdit,
  FollowUpSummary,
  Recipients,
} from '@/lib/ipc/generated/followups';

/** The editor's fields while the user types: addresses as typed, comma-separated. */
export interface DraftFields {
  subject: string;
  body: string;
  to: string;
  cc: string;
  bcc: string;
}

/** One of Home's suggestions, shaped like ORG-02's `SuggestedFollowUp` slot item. */
export interface FollowUpSuggestion {
  id: string;
  title: string;
  meetingId: MeetingId;
}
