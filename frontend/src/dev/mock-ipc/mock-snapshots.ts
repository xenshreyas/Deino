/**
 * The mock's screen snapshots: with `?screen=allowed`, every meeting has
 * the fixture slides (`fixtures/screen/*.png`) as snapshots, for the
 * Images strip's screenshots. Deleting one hides it until a reload.
 */
import type * as Generated from '@/lib/ipc/generated/screen';

/** The fixture slides' URLs in the dev server, keyed by repository path. */
const slides = import.meta.glob<string>('../../../../fixtures/screen/*.png', {
  eager: true,
  query: '?url',
  import: 'default',
});

/** The folder the mock gives snapshot paths, so the asset mock knows them. */
export const SNAPSHOT_FOLDER = '/mock-snapshots';

const deleted = new Set<number>();

/** Whether the page asked to start granted (`?screen=allowed`). */
export function startsGranted(): boolean {
  return new URLSearchParams(window.location.search).get('screen') === 'allowed';
}

/** The fixture slides as snapshots, 12 s apart. */
export function fixtureSnapshots(): Generated.Snapshot[] {
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

/** Hides snapshot `id`. */
export function deleteSnapshot(id: number): void {
  deleted.add(id);
}

/** The dev-server URL `convertFileSrc` answers for a mock snapshot path. */
export function snapshotUrl(path: string): string | undefined {
  if (!path.startsWith(`${SNAPSHOT_FOLDER}/`)) return undefined;
  const name = path.slice(SNAPSHOT_FOLDER.length + 1);
  return Object.entries(slides).find(([fixture]) => fixture.endsWith(`/${name}`))?.[1];
}
