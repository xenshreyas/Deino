/**
 * The mock recordings folder: the fixtures' audio (`fixtures/audio/<meeting>/
 * {mic,system}.m4a`, the first ~55 s of the standup and the 1:1, spoken by
 * macOS voices into each segment's own slot) served by Vite, so the player
 * plays real audio whose timestamps the transcript knows. A fixture's audio
 * appears as the meeting's Me and Them tracks; there is no mixed file.
 */
import type { MeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/player';

import type { FixtureMeeting } from './fixture-schema';

/** Every fixture audio file's URL in the dev server, keyed by repository path. */
const urls = import.meta.glob<string>('../../../../fixtures/audio/*/*.m4a', {
  eager: true,
  query: '?url',
  import: 'default',
});

/** The prefix the mock gives recording paths, so the asset mock can tell them apart. */
const RECORDINGS = '/mock-recordings/';

/** The URL of a fixture file (`audio/standup/mic.m4a`), if it is bundled. */
function urlOf(fixturePath: string): string | undefined {
  const entry = Object.entries(urls).find(([path]) => path.endsWith(`/fixtures/${fixturePath}`));
  return entry?.[1];
}

/** Each fixture meeting's recording files. */
export class MockMedia {
  private readonly files = new Map<string, Generated.MediaFile[]>();

  constructor(fixtures: readonly FixtureMeeting[]) {
    for (const fixture of fixtures) {
      if (fixture.audio === null) continue;
      this.files.set(fixture.id, [
        { source: 'microphone', path: `${RECORDINGS}${fixture.audio.mic}` },
        { source: 'system', path: `${RECORDINGS}${fixture.audio.system}` },
      ]);
    }
  }

  /** `player|media`. */
  media(meetingId: MeetingId): Generated.MeetingMedia {
    return { files: this.files.get(meetingId) ?? [] };
  }

  /** The dev-server URL `convertFileSrc` answers for a mock recording path. */
  static url(path: string): string | undefined {
    return path.startsWith(RECORDINGS) ? urlOf(path.slice(RECORDINGS.length)) : undefined;
  }
}
