/**
 * Mock of `@/lib/ipc/generated/screen`: Screen Recording is missing until
 * "Open System Settings" is pressed, which grants it, so `pnpm dev:mock`
 * shows both states. `?screen=allowed` starts granted and gives every
 * meeting the fixture slides (`fixtures/screen/*.png`) as snapshots, for
 * the Images strip's screenshots; deleting one hides it until a reload.
 * No capture runs.
 *
 * It imports no feature code: the feature reaches this module through its
 * generated bindings, so importing it back would form a cycle.
 */
import type * as Generated from '@/lib/ipc/generated/screen';

import { ok } from './command-result';
import { latency } from './latency';

/** The fixture slides' URLs in the dev server, keyed by repository path. */
const slides = import.meta.glob<string>('../../../../fixtures/screen/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
});

/** The folder the mock gives snapshot paths, so the asset mock knows them. */
const FOLDER = '/mock-snapshots';

let permitted: boolean | null = null;
const deleted = new Set<number>();

/** Whether the page asked to start granted (`?screen=allowed`). */
function startsGranted(): boolean {
  return new URLSearchParams(window.location.search).get('screen') === 'allowed';
}

/** The fixture slides as snapshots, 12 s apart. */
function fixtureSnapshots(): Generated.Snapshot[] {
  if (!startsGranted()) return [];
  return Object.keys(slides)
    .sort()
    .map((path, index) => ({
      id: index + 1,
      at: 12_000 * (index + 1),
      fileName: path.split('/').pop() ?? '',
      width: 960,
      height: 540,
      text: null,
    }))
    .filter((snapshot) => !deleted.has(snapshot.id));
}

/** The dev-server URL `convertFileSrc` answers for a mock snapshot path. */
export function snapshotUrl(path: string): string | undefined {
  if (!path.startsWith(`${FOLDER}/`)) return undefined;
  const name = path.slice(FOLDER.length + 1);
  return Object.entries(slides).find(([fixture]) => fixture.endsWith(`/${name}`))?.[1];
}

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
    return ok(FOLDER);
  },
  async deleteSnapshot(_meetingId: Generated.MeetingId, snapshotId: number) {
    await latency();
    deleted.add(snapshotId);
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
