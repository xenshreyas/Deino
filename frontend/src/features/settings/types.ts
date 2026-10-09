/**
 * The settings feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { Settings } from '@/lib/ipc/generated/settings';

export type {
  AiSettings,
  CustomOpenAiSettings,
  LlmProvider,
  NotificationSettings,
  Settings,
  Theme,
  TranscriptionEngine,
  TranscriptionSettings,
} from '@/lib/ipc/generated/settings';
export type { SecretName, SecretStatus } from '@/lib/ipc/generated/secrets';

/**
 * A partial value for a merge patch: objects recurse, everything else
 * (including arrays, which a merge patch replaces whole) is taken as is.
 */
export type PatchOf<T> = T extends readonly unknown[]
  ? T
  : T extends object
    ? { [K in keyof T]?: PatchOf<T[K]> }
    : T;

/**
 * An RFC 7396 merge patch for {@link Settings}: name only what changes.
 * `null` on an optional setting resets it; `version` is read-only.
 */
export type SettingsPatch = PatchOf<Omit<Settings, 'version'>>;
