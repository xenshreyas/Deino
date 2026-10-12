/**
 * The screen feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
export type {
  CaptureState,
  CaptureStatus,
  MeetingId,
  ScreenStatus,
  Snapshot,
  Source as CaptureSource,
  TargetChoice,
} from '@/lib/ipc/generated/screen';
export type { ScreenSettings } from '@/lib/ipc/generated/settings';
