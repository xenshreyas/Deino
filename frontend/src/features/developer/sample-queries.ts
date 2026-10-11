/**
 * The debug-only sample seeding. Seeding writes through Rust's writer
 * connection, so the `DbChanged` that follows refreshes the meeting list and
 * every open meeting on its own; the mutation does not touch the cache.
 */
import { useMutation } from '@tanstack/react-query';

import { sampleApi } from './sample-api';

/** Loads the fixture meetings into the database. */
export function useSeedFixtures() {
  return useMutation({ mutationFn: sampleApi.seedFixtures });
}
