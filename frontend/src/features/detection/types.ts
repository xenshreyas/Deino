/**
 * The detection feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { BundleId } from '@/lib/ipc/generated/detection';

export type {
  AppIdentity,
  BundleId,
  Detection,
  DetectionId,
  DetectionOverview,
  DetectionSettings,
  IgnoredApp,
  LinkedEvent,
  MeetingKind,
  RecordingRequested,
} from '@/lib/ipc/generated/detection';

/**
 * Brands a bundle identifier that arrived as plain text (fixtures, tests).
 * Rust validates real ones; the only check here is that there is one.
 */
export function asBundleId(raw: string): BundleId {
  if (raw.trim() === '') throw new TypeError('A bundle identifier cannot be empty.');
  return raw as BundleId;
}
