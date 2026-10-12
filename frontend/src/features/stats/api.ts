/**
 * Typed calls to the `stats` plugin. Each resolves to the data or throws an
 * `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/stats';
import { unwrap } from '@/lib/ipc/result';

import type { Heatmap, RecapStatus, StatsOverview, StatsRange, YearRecap } from './types';

export const statsApi = {
  /** The numbers of `range`, ending now. */
  overview: (range: StatsRange): Promise<StatsOverview> => unwrap(commands.overview(range)),
  /** Meetings per day over the last year. */
  heatmap: (): Promise<Heatmap> => unwrap(commands.heatmap()),
  /** This year's recap, if generated, and whether it can be. */
  recapStatus: (): Promise<RecapStatus> => unwrap(commands.recap()),
  /** Writes this year's recap (once); rejects outside December or without a model. */
  generateRecap: (): Promise<YearRecap> => unwrap(commands.writeRecap()),
};
