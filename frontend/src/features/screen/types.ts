/**
 * The screen feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
export type {
  CaptureState,
  CaptureStatus,
  ScreenStatus,
  Snapshot,
} from '@/lib/ipc/generated/screen';
export type { ScreenSettings } from '@/lib/ipc/generated/settings';
