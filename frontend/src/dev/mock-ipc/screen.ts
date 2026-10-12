/**
 * Mock of `@/lib/ipc/generated/screen`: Screen Recording is missing until
 * "Open System Settings" is pressed, which grants it, so `pnpm dev:mock`
 * shows both states. `?screen=allowed` starts granted and gives every
 * meeting the fixture slides as snapshots (`mock-snapshots.ts`). No
 * capture runs.
 *
 * It imports no feature code: the feature reaches this module through its
 * generated bindings, so importing it back would form a cycle.
 */
import type * as Generated from '@/lib/ipc/generated/screen';

import { ok } from './command-result';
import { latency } from './latency';
import { SNAPSHOT_FOLDER, deleteSnapshot, fixtureSnapshots, startsGranted } from './mock-snapshots';

let permitted: boolean | null = null;

export const commands = {
  async status() {
    await latency();
    permitted ??= startsGranted();
    return ok({
      supported: true,
      permitted,
      capture: { meeting: null, state: 'off' as const, video: false, snapshots: 0 },
    });
  },
  async openScreenRecordingSettings() {
    await latency();
    permitted = true;
    return ok(null);
  },
  async turnOffForMeeting() {
    await latency();
    return ok(false);
  },
  async snapshots() {
    await latency();
    return ok(fixtureSnapshots());
  },
  async snapshotFolder() {
    await latency();
    return ok(SNAPSHOT_FOLDER);
  },
  async deleteSnapshot(_meetingId: Generated.MeetingId, snapshotId: number) {
    await latency();
    deleteSnapshot(snapshotId);
    return ok(true);
  },
  async captureChoices() {
    await latency();
    return ok([{ source: { kind: 'display' as const, id: 1 }, label: 'Entire screen' }]);
  },
  async captureSource() {
    await latency();
    return ok(true);
  },
  async consentReminder() {
    await latency();
    return ok(false);
  },
  async stopConsentReminders() {
    await latency();
    return ok(null);
  },
} satisfies typeof Generated.commands;
