/**
 * The fixture meetings the smoke tests expect to see: the same
 * `fixtures/meetings/*.json` that `pnpm dev:mock` serves, validated with the
 * same zod schema.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { type FixtureMeeting, fixtureMeetingSchema } from '../src/dev/mock-ipc/fixture-schema';

// Playwright loads tests as CommonJS here (package.json has no "type": "module").
const MEETINGS_DIR = join(__dirname, '../../fixtures/meetings');

/** Every fixture meeting, in file-name order. */
export function fixtureMeetings(): FixtureMeeting[] {
  return readdirSync(MEETINGS_DIR)
    .filter((file) => file.endsWith('.json'))
    .sort()
    .map((file) => {
      const json: unknown = JSON.parse(readFileSync(join(MEETINGS_DIR, file), 'utf8'));
      return fixtureMeetingSchema.parse(json);
    });
}

/** The fixture with `id`; throws when it is missing. */
export function fixtureMeeting(id: string): FixtureMeeting {
  const meeting = fixtureMeetings().find((candidate) => candidate.id === id);
  if (!meeting) throw new Error(`no fixture meeting ${id}`);
  return meeting;
}
