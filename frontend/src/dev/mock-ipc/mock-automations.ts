/**
 * Automations for `pnpm dev:mock`: an in-memory twin of
 * `deino_core::automations` (unique names ignoring case, at least one
 * step, switched-off duplicates, one latest run per automation and
 * meeting) reporting writes as `DbChanged` tables. A run finishes a moment
 * later with every step done, like the pipeline stage.
 */
import type * as Generated from '@/lib/ipc/generated/automations';
import type { AppError, Table } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import type { FixtureMeeting } from './fixture-schema';
import { mockMatches } from './mock-insight-answers';

type Automation = Generated.Automation;
type Run = Generated.AutomationRun;
type AutomationId = Generated.AutomationId;
type MeetingId = Generated.MeetingId;

/** How long the mock stage takes per run. */
const RUN_MS = 600;
/** `deino_core::automations::MAX_RECENT_RUNS`. */
const MAX_RECENT_RUNS = 10;
/** What the mock writes report. */
const TABLES: Table[] = ['automations', 'automation_runs', 'automation_run_steps'];

function notFound(what: string): AppError {
  return { code: 'not_found', message: `${what} was not found` };
}

/** The automations and their runs, with change notifications. */
export class MockAutomations {
  private automations: Automation[] = [];
  private runs: Run[] = [];

  constructor(
    private readonly meetings: readonly FixtureMeeting[],
    private readonly onChange: (tables: Table[]) => void,
  ) {}

  list(): Automation[] {
    return this.automations;
  }

  get(id: AutomationId): CommandResult<Automation> {
    const found = this.automations.find((automation) => automation.id === id);
    return found === undefined ? failure(notFound(`automation ${id}`)) : ok(found);
  }

  save(
    id: AutomationId | null,
    input: Generated.AutomationInput,
    now: Date,
  ): CommandResult<Automation> {
    const name = input.name.trim();
    if (name === '' || input.steps.length === 0) {
      return failure({ code: 'invalid', message: 'name and at least one step are required' });
    }
    const taken = this.automations.some(
      (other) => other.id !== id && other.name.toLowerCase() === name.toLowerCase(),
    );
    if (taken)
      return failure({ code: 'conflict', message: 'an automation with this name already exists' });
    const current = id === null ? undefined : this.automations.find((other) => other.id === id);
    const at = now.toISOString();
    const automation: Automation = {
      ...input,
      name,
      id: current?.id ?? (crypto.randomUUID() as AutomationId),
      createdAt: current?.createdAt ?? at,
      updatedAt: at,
    };
    this.automations = [
      ...this.automations.filter((other) => other.id !== automation.id),
      automation,
    ];
    this.onChange(TABLES);
    return ok(automation);
  }

  delete(id: AutomationId): CommandResult<null> {
    if (!this.automations.some((automation) => automation.id === id)) {
      return failure(notFound(`automation ${id}`));
    }
    this.automations = this.automations.filter((automation) => automation.id !== id);
    this.runs = this.runs.filter((run) => run.automationId !== id);
    this.onChange(TABLES);
    return ok(null);
  }

  duplicate(id: AutomationId, now: Date): CommandResult<Automation> {
    const found = this.get(id);
    if (found.status === 'error') return found;
    const { name, autoRun, stopOnFailure, conditions, steps } = found.data;
    let copy = `${name} (copy)`;
    for (let n = 2; this.automations.some((other) => other.name === copy); n += 1) {
      copy = `${name} (copy ${String(n)})`;
    }
    return this.save(
      null,
      { name: copy, enabled: false, autoRun, stopOnFailure, conditions, steps },
      now,
    );
  }

  forMeeting(meetingId: MeetingId): CommandResult<Generated.MeetingAutomation[]> {
    const meeting = this.meetings.find((candidate) => candidate.id === meetingId);
    if (meeting === undefined) return failure(notFound(`meeting ${meetingId}`));
    return ok(
      this.automations.map((automation) => ({
        automation,
        matches: mockMatches(automation.conditions, meeting),
        run: this.runOf(automation.id, meetingId),
      })),
    );
  }

  matching(id: AutomationId): CommandResult<Generated.MatchingMeeting[]> {
    const found = this.get(id);
    if (found.status === 'error') return found;
    return ok(
      [...this.meetings]
        .filter((meeting) => mockMatches(found.data.conditions, meeting))
        .sort((a, b) => b.startedAt.localeCompare(a.startedAt))
        .map((meeting) => ({
          meeting: {
            id: meeting.id as MeetingId,
            title: meeting.title,
            startedAt: meeting.startedAt,
          },
          run: this.runOf(id, meeting.id as MeetingId),
        })),
    );
  }

  /** The run log: the automation's latest runs, newest first. */
  recentRuns(id: AutomationId): CommandResult<Generated.RecentRun[]> {
    const found = this.get(id);
    if (found.status === 'error') return found;
    return ok(
      this.runs
        .filter((run) => run.automationId === id)
        .sort((a, b) => b.requestedAt.localeCompare(a.requestedAt))
        .slice(0, MAX_RECENT_RUNS)
        .map((run) => ({
          meetingTitle: this.meetings.find((meeting) => meeting.id === run.meetingId)?.title ?? '',
          run,
        })),
    );
  }

  run(id: AutomationId, meetingIds: MeetingId[], now: Date): CommandResult<null> {
    const found = this.get(id);
    if (found.status === 'error') return found;
    for (const meetingId of meetingIds) this.start(found.data, meetingId, now);
    return ok(null);
  }

  /** The newest fixture meeting, run on. */
  testLatest(id: AutomationId, now: Date): CommandResult<MeetingId> {
    const latest = [...this.meetings].sort((a, b) => b.startedAt.localeCompare(a.startedAt))[0];
    if (latest === undefined) return failure(notFound('a finished meeting'));
    const ran = this.run(id, [latest.id as MeetingId], now);
    return ran.status === 'error' ? ran : ok(latest.id as MeetingId);
  }

  private runOf(id: AutomationId, meetingId: MeetingId): Run | null {
    return this.runs.find((run) => run.automationId === id && run.meetingId === meetingId) ?? null;
  }

  /** A pending run that finishes a moment later. */
  private start(automation: Automation, meetingId: MeetingId, now: Date) {
    const run: Run = {
      id: crypto.randomUUID() as Generated.AutomationRunId,
      automationId: automation.id,
      meetingId,
      trigger: { kind: 'manual' },
      status: 'pending',
      steps: [],
      requestedAt: now.toISOString(),
      finishedAt: null,
    };
    this.runs = [
      ...this.runs.filter((other) => other !== this.runOf(automation.id, meetingId)),
      run,
    ];
    this.onChange(TABLES);
    setTimeout(() => {
      this.finish(automation, run);
    }, RUN_MS);
  }

  private finish(automation: Automation, run: Run) {
    const at = new Date().toISOString();
    const steps: Generated.RunStep[] = automation.steps.map((action, position) => ({
      position,
      kind: action.kind,
      status: 'done',
      failure: null,
      error: null,
      output: { kind: 'notified' },
      finishedAt: at,
    }));
    this.runs = this.runs.map((other) =>
      other.id === run.id ? { ...run, status: 'done', steps, finishedAt: at } : other,
    );
    this.onChange(TABLES);
  }
}
