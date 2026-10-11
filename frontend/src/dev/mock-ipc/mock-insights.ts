/**
 * Insights for `pnpm dev:mock`: an in-memory twin of `deino_core::insights`
 * with its rules (unique names ignoring case, revisions bumped by edits of
 * what an insight asks, pending runs that keep earlier answers), reporting
 * writes as `DbChanged` tables. A run "answers" each meeting a moment later,
 * one at a time, like the pipeline stage.
 */
import type * as Generated from '@/lib/ipc/generated/insights';
import type { AppError, Table } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import type { FixtureMeeting } from './fixture-schema';
import { mockAnswers, mockMatches } from './mock-insight-answers';

type Def = Generated.InsightDef;
type Run = Generated.InsightRun;
type InsightId = Generated.InsightId;
type MeetingId = Generated.MeetingId;

/** How long the mock model takes per meeting. */
const ANSWER_MS = 500;

function notFound(what: string): AppError {
  return { code: 'not_found', message: `${what} was not found` };
}

/** True when `input` asks something other than `def` does. */
function asksDifferently(def: Def, input: Generated.InsightInput): boolean {
  const { name, description, fields, cardinality } = input;
  return (
    JSON.stringify([def.name, def.description, def.fields, def.cardinality]) !==
    JSON.stringify([name, description, fields, cardinality])
  );
}

/** The definitions and runs, with change notifications. */
export class MockInsights {
  private defs: Def[] = [];
  private runs: Run[] = [];
  private queue: Promise<void> = Promise.resolve();

  constructor(
    private readonly meetings: readonly FixtureMeeting[],
    private readonly onChange: (tables: Table[]) => void,
  ) {}

  list(): Generated.InsightSummary[] {
    return this.defs.map((def) => ({ def, progress: this.progress(def.id) }));
  }

  get(id: InsightId): CommandResult<Def> {
    const def = this.defs.find((candidate) => candidate.id === id);
    return def === undefined ? failure(notFound(`insight ${id}`)) : ok(def);
  }

  save(id: InsightId | null, input: Generated.InsightInput, now: Date): CommandResult<Def> {
    const name = input.name.trim();
    if (name === '' || input.description.trim() === '') {
      return failure({
        code: 'invalid',
        message: name === '' ? 'name cannot be empty' : 'description cannot be empty',
      });
    }
    if (this.defs.some((def) => def.id !== id && def.name.toLowerCase() === name.toLowerCase())) {
      return failure({ code: 'conflict', message: 'an insight with this name already exists' });
    }
    const current = this.defs.find((def) => def.id === id);
    if (id !== null && current === undefined) return failure(notFound(`insight ${id}`));
    const def: Def = {
      ...input,
      name,
      id: current?.id ?? (crypto.randomUUID() as InsightId),
      revision:
        current === undefined ? 1 : current.revision + (asksDifferently(current, input) ? 1 : 0),
      createdAt: current?.createdAt ?? now.toISOString(),
      updatedAt: now.toISOString(),
    };
    this.defs =
      current === undefined
        ? [...this.defs, def]
        : this.defs.map((d) => (d.id === def.id ? def : d));
    this.onChange(['insight_defs']);
    return ok(def);
  }

  delete(id: InsightId): CommandResult<null> {
    if (!this.defs.some((def) => def.id === id)) return failure(notFound(`insight ${id}`));
    this.defs = this.defs.filter((def) => def.id !== id);
    this.runs = this.runs.filter((run) => run.insightId !== id);
    this.onChange(['insight_defs', 'insight_results']);
    return ok(null);
  }

  forMeeting(meetingId: MeetingId): CommandResult<Generated.MeetingInsights> {
    if (!this.meetings.some((meeting) => meeting.id === meetingId))
      return failure(notFound(`meeting ${meetingId}`));
    const entries: Generated.MeetingInsight[] = [];
    const others: Def[] = [];
    for (const def of this.defs) {
      const run = this.runs.find(
        (candidate) => candidate.insightId === def.id && candidate.meetingId === meetingId,
      );
      if (run === undefined) others.push(def);
      else entries.push({ def, run, stale: run.revision !== null && run.revision < def.revision });
    }
    return ok({ entries, others });
  }

  table(id: InsightId): CommandResult<Generated.InsightTable> {
    const found = this.get(id);
    if (found.status === 'error') return found;
    const rows = this.meetings.flatMap((meeting) => {
      const run = this.runs.find(
        (candidate) => candidate.insightId === id && candidate.meetingId === meeting.id,
      );
      return run === undefined
        ? []
        : [{ meetingTitle: meeting.title, meetingStartedAt: meeting.startedAt, run }];
    });
    rows.sort((a, b) => b.meetingStartedAt.localeCompare(a.meetingStartedAt));
    return ok({ def: found.data, rows, progress: this.progress(id) });
  }

  candidates(id: InsightId): CommandResult<Generated.RunCandidate[]> {
    const found = this.get(id);
    if (found.status === 'error') return found;
    const apply = found.data.applyWhen;
    const candidates = [...this.meetings]
      .sort((a, b) => b.startedAt.localeCompare(a.startedAt))
      .map((meeting) => ({
        meeting: {
          id: meeting.id as MeetingId,
          title: meeting.title,
          startedAt: meeting.startedAt,
        },
        matches: apply.mode === 'automatic' && mockMatches(apply.conditions, meeting),
        status:
          this.runs.find((run) => run.insightId === id && run.meetingId === meeting.id)?.status ??
          null,
      }));
    return ok(candidates);
  }

  run(
    id: InsightId,
    meetingIds: readonly MeetingId[],
    now: Date,
  ): CommandResult<Generated.RunProgress> {
    const found = this.get(id);
    if (found.status === 'error') return found;
    if (meetingIds.length === 0)
      return failure({ code: 'invalid', message: 'meetings must number 1 to 500' });
    for (const meetingId of new Set(meetingIds)) {
      this.upsert(id, meetingId, {
        status: 'pending',
        error: null,
        requestedAt: now.toISOString(),
      });
      this.queue = this.queue.then(() => this.answer(id, meetingId));
    }
    this.onChange(['insight_results']);
    return ok(this.progress(id));
  }

  /** The answers the mock "model" gives, a moment later. */
  private async answer(id: InsightId, meetingId: MeetingId): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, ANSWER_MS));
    const def = this.defs.find((candidate) => candidate.id === id);
    const meeting = this.meetings.find((candidate) => candidate.id === meetingId);
    if (def === undefined || meeting === undefined) return;
    const at = new Date().toISOString();
    this.upsert(id, meetingId, {
      status: 'done',
      answers: mockAnswers(def, meeting),
      revision: def.revision,
      generatedAt: at,
    });
    this.onChange(['insight_results']);
  }

  private upsert(id: InsightId, meetingId: MeetingId, change: Partial<Run>): void {
    const index = this.runs.findIndex((run) => run.insightId === id && run.meetingId === meetingId);
    const base: Run = this.runs[index] ?? {
      insightId: id,
      meetingId,
      status: 'pending',
      answers: [],
      revision: null,
      error: null,
      requestedAt: new Date().toISOString(),
      generatedAt: null,
    };
    const next = { ...base, ...change };
    this.runs =
      index < 0 ? [...this.runs, next] : this.runs.map((run, at) => (at === index ? next : run));
  }

  private progress(id: InsightId): Generated.RunProgress {
    const runs = this.runs.filter((run) => run.insightId === id);
    const count = (status: Generated.InsightStatus) =>
      runs.filter((run) => run.status === status).length;
    return { pending: count('pending'), done: count('done'), failed: count('failed') };
  }

  /** The answered insights of a meeting, for the mock export preview. */
  answeredOn(meetingId: string): { name: string; answers: Generated.InsightAnswer[] }[] {
    return this.defs.flatMap((def) => {
      const run = this.runs.find(
        (candidate) => candidate.insightId === def.id && candidate.meetingId === meetingId,
      );
      return run === undefined || run.answers.length === 0
        ? []
        : [{ name: def.name, answers: run.answers }];
    });
  }
}
