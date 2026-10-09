/**
 * The repository's fixture meetings (`fixtures/meetings/*.json`), bundled
 * into the mock build and validated with zod as they load.
 *
 * Only `pnpm dev:mock`, Vitest and Playwright import this; production builds
 * never reference `src/dev/`, so the fixtures never ship.
 */
import { type FixtureMeeting, fixtureMeetingSchema } from './fixture-schema';

/** Every fixture file, keyed by path, parsed as JSON by Vite. */
const files = import.meta.glob<unknown>('../../../../fixtures/meetings/*.json', {
  eager: true,
  import: 'default',
});

/** Validates every fixture file; a broken one fails loudly, naming the file. */
export function loadFixtureMeetings(): FixtureMeeting[] {
  return Object.entries(files)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([path, json]) => {
      const parsed = fixtureMeetingSchema.safeParse(json);
      if (!parsed.success) {
        throw new Error(`${path} does not match the fixture schema: ${parsed.error.message}`);
      }
      return parsed.data;
    });
}
