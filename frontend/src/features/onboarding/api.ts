/**
 * Typed calls to the `onboarding` plugin: the macOS permissions setup asks
 * for, and the bundled sample meeting. Progress is a settings section (`settingsApi`), and every other step
 * calls the feature that owns it.
 */
import { commands, type MeetingId } from '@/lib/ipc/generated/onboarding';
import { unwrap } from '@/lib/ipc/result';

import type { PermissionKind, PermissionStatus } from './types';

export const onboardingApi = {
  /** Microphone, system audio, notifications and Accessibility, read without prompting. */
  permissions: (): Promise<PermissionStatus[]> => unwrap(commands.permissions()),
  /** Shows the macOS prompt for `kind` when it was never answered; the status afterwards. */
  requestPermission: (kind: PermissionKind): Promise<PermissionStatus> =>
    unwrap(commands.requestPermission(kind)),
  /** Opens the System Settings page where `kind` is granted. */
  openPermissionSettings: (kind: PermissionKind): Promise<null> =>
    unwrap(commands.openPermissionSettings(kind)),
  /** Adds the bundled sample meeting, or finds the one already there; its ID. */
  importSample: (): Promise<MeetingId> => unwrap(commands.importSample()),
};
