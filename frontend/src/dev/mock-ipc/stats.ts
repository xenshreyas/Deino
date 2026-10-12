/**
 * Mock of `@/lib/ipc/generated/stats`: the Stats page over the fixtures and
 * a deterministic year of history (`mock-stats.ts`). The year in review is
 * open, as with the app's `DEINO_RECAP_ANYTIME=1` debug flag, and writing
 * it returns a canned recap; dev:mock has no LLM.
 */
import type * as Generated from '@/lib/ipc/generated/stats';

import { fixtureMeetings } from './database';
import { latency } from './latency';
import { ok } from './mock-database';
import { heatmapOf, mockFacts, overviewOf, sinceOf } from './mock-stats';

const now = new Date();
const facts = mockFacts(fixtureMeetings, now);
let recap: Generated.YearRecap | null = null;

const RECAP = `A year of steady building: most of your time went to the people shipping Deino with you, and Mondays carried the week.

### The rhythm
Standups and planning set the pace, with the busiest stretch in late September as Q4 planning, the board update and the onboarding brainstorm landed in one week.

### The people
Priya, Tom and Maya were in more of your meetings than anyone else.

### Outside the team
Brightkite Logistics, Fernhollow and Quarrystone kept the outside conversations going: discovery, support and the board.

Next year starts with the Q4 plan already in motion.`;

export const commands = {
  async overview(range) {
    await latency();
    return ok(overviewOf(facts, sinceOf(range, now)));
  },
  async heatmap() {
    await latency();
    return ok(heatmapOf(facts, now));
  },
  async recapStatus() {
    await latency();
    return ok({ year: now.getUTCFullYear(), available: true, recap });
  },
  async generateRecap() {
    await latency();
    recap ??= {
      year: now.getUTCFullYear(),
      markdown: RECAP,
      generatedAt: now.toISOString(),
    };
    return ok(recap);
  },
} satisfies typeof Generated.commands;
