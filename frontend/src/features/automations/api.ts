/**
 * Typed calls to the `automations` plugin. Each resolves to the data or
 * throws an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/automations';
import { unwrap } from '@/lib/ipc/result';

import type {
  Automation,
  AutomationId,
  AutomationInput,
  MatchingMeeting,
  MeetingAutomation,
  MeetingId,
  StepInfo,
} from './types';

export const automationsApi = {
  /** Every kind of step, ready or "WIP", in picker order. */
  catalog: (): Promise<StepInfo[]> => unwrap(commands.catalog()),
  /** Every automation, oldest first. */
  list: (): Promise<Automation[]> => unwrap(commands.list()),
  /** One automation. */
  get: (id: AutomationId): Promise<Automation> => unwrap(commands.get(id)),
  /** Creates an automation; rejects with `invalid` or `conflict` (a taken name). */
  create: (input: AutomationInput): Promise<Automation> => unwrap(commands.create(input)),
  /** Replaces an automation's settings, conditions and steps. */
  update: (id: AutomationId, input: AutomationInput): Promise<Automation> =>
    unwrap(commands.update(id, input)),
  /** Deletes an automation and its runs. */
  delete: (id: AutomationId): Promise<null> => unwrap(commands.delete(id)),
  /** A switched-off copy named "… (copy)". */
  duplicate: (id: AutomationId): Promise<Automation> => unwrap(commands.duplicate(id)),
  /** A meeting's ⋯ → Automations menu. */
  forMeeting: (meetingId: MeetingId): Promise<MeetingAutomation[]> =>
    unwrap(commands.forMeeting(meetingId)),
  /** The finished meetings an automation matches, with its runs there. */
  matching: (id: AutomationId): Promise<MatchingMeeting[]> => unwrap(commands.matching(id)),
  /** Runs (or reruns) an automation on the picked meetings. */
  run: (id: AutomationId, meetingIds: MeetingId[]): Promise<null> =>
    unwrap(commands.run(id, meetingIds)),
  /** Runs an automation on the newest finished meeting; resolves to that meeting. */
  testLatest: (id: AutomationId): Promise<MeetingId> => unwrap(commands.testLatest(id)),
};
