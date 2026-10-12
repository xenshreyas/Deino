/**
 * The onboarding feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
export type { OnboardingSettings, OnboardingStep, StepOutcome } from '@/lib/ipc/generated/settings';
export type {
  PermissionKind,
  PermissionState,
  PermissionStatus,
} from '@/lib/ipc/generated/onboarding';
