/**
 * The tags feature's types, generated from Rust by `bindings.rs`, plus the
 * brand checks for IDs that arrive as plain strings (URL search params).
 */
import type { TagId } from '@/lib/ipc/generated/tags';

export type {
  AppliedTag,
  MeetingId,
  MeetingTagged,
  MeetingTagLink,
  NewTag,
  PaletteColor,
  Tag,
  TagId,
  TagSource,
  TagUpdate,
} from '@/lib/ipc/generated/tags';

/** A tag ID from a URL. Tag IDs are UUIDs minted by Rust. */
export function asTagId(raw: string): TagId {
  if (raw.trim() === '') throw new TypeError('A tag ID cannot be empty.');
  return raw as TagId;
}
