/**
 * Mock of `@/lib/ipc/generated/onboarding`: a Mac where only the microphone
 * is allowed yet. Asking grants a permission; System Settings "opens"
 * without changing anything, as on a real Mac until the user flips a switch.
 * The sample meeting is the standup fixture it was cut from.
 */
import type * as Generated from '@/lib/ipc/generated/onboarding';

import { ok } from './command-result';
import { latency } from './latency';

type PermissionKind = Generated.PermissionKind;
type PermissionStatus = Generated.PermissionStatus;

/** The fixture meeting the sample's transcript was cut from (Rust's `sample`). */
const SAMPLE_STAND_IN = 'meeting-fixture-standup' as Generated.MeetingId;

/** Each permission's state, in the order Rust lists them. */
const states = new Map<PermissionKind, Generated.PermissionState>([
  ['microphone', 'granted'],
  ['system_audio', 'unknown'],
  ['notifications', 'not_determined'],
  ['accessibility', 'not_determined'],
]);

/** The status the real service would report for `kind`. */
function status(kind: PermissionKind): PermissionStatus {
  return { kind, state: states.get(kind) ?? 'unknown', optional: kind === 'accessibility' };
}

export const commands = {
  async permissions() {
    await latency();
    return ok([...states.keys()].map(status));
  },
  async requestPermission(kind) {
    await latency();
    states.set(kind, 'granted');
    return ok(status(kind));
  },
  async openPermissionSettings() {
    await latency();
    return ok(null);
  },
  /** The mock has no importer: the standup fixture stands in for the sample. */
  async importSample() {
    await latency();
    return ok(SAMPLE_STAND_IN);
  },
} satisfies typeof Generated.commands;
