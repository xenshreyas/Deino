/**
 * The meetings feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { MeetingId } from '@/lib/ipc/generated/meetings';

export type { Meeting, MeetingId } from '@/lib/ipc/generated/meetings';

/** Input of the rename mutation. */
export interface RenameMeetingInput {
  id: MeetingId;
  title: string;
}

/**
 * Brands a meeting ID that arrived as a plain string (a route param, a legacy
 * command's result). Meeting IDs are opaque text minted by Rust, so the only
 * check is that there is one.
 */
export function asMeetingId(raw: string): MeetingId {
  if (raw.trim() === '') throw new TypeError('A meeting ID cannot be empty.');
  return raw as MeetingId;
}
