/**
 * Typed calls to the `insights` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/insights';
import { unwrap } from '@/lib/ipc/result';

import type {
  InsightDef,
  InsightId,
  InsightInput,
  InsightSummary,
  InsightTable,
  InsightTemplate,
  MeetingId,
  MeetingInsights,
  RunCandidate,
  RunProgress,
  SavedExport,
} from './types';

export const insightsApi = {
  /** The twelve ready-made insights to start from. */
  templates: (): Promise<InsightTemplate[]> => unwrap(commands.templates()),
  /** Every insight, oldest first, with where its runs stand. */
  list: (): Promise<InsightSummary[]> => unwrap(commands.list()),
  /** One insight. */
  get: (id: InsightId): Promise<InsightDef> => unwrap(commands.get(id)),
  /** Defines an insight; rejects with `invalid` or `conflict` (a taken name). */
  create: (input: InsightInput): Promise<InsightDef> => unwrap(commands.create(input)),
  /** Replaces an insight's definition. */
  update: (id: InsightId, input: InsightInput): Promise<InsightDef> =>
    unwrap(commands.update(id, input)),
  /** Deletes an insight and its answers. */
  delete: (id: InsightId): Promise<null> => unwrap(commands.delete(id)),
  /** A meeting's Insights tab. */
  forMeeting: (meetingId: MeetingId): Promise<MeetingInsights> =>
    unwrap(commands.forMeeting(meetingId)),
  /** One insight's answers across meetings. */
  table: (id: InsightId): Promise<InsightTable> => unwrap(commands.table(id)),
  /** The past meetings an insight could run on, newest first. */
  candidates: (id: InsightId): Promise<RunCandidate[]> => unwrap(commands.candidates(id)),
  /** Runs an insight on the picked meetings in the background. */
  run: (id: InsightId, meetingIds: MeetingId[]): Promise<RunProgress> =>
    unwrap(commands.run(id, meetingIds)),
  /** Saves an insight's table as CSV; `null` when the user cancels. */
  exportCsv: (id: InsightId): Promise<SavedExport | null> => unwrap(commands.exportCsv(id)),
};
