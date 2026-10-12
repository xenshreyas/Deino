/**
 * Mock of `@/lib/ipc/generated/clips`: clips in memory with the Rust rules
 * (a name and a range are required; file names are made safe and unique in
 * the meeting's `clips/` folder), one seeded on the stand-up fixture so the
 * Clips tab and ⌘K have something to show. A browser has no Finder, so
 * Reveal only answers.
 */
import type * as Generated from '@/lib/ipc/generated/clips';
import type { Table } from '@/lib/ipc/generated/shared';

import { failure, ok } from './command-result';
import { fixtureMeetings } from './database';
import { events } from './db';
import { latency } from './latency';

type Clip = Generated.Clip;

const STANDUP = 'meeting-fixture-standup' as Generated.MeetingId;
const SEEDED_AT = '2026-10-08T09:30:00.000Z';
const CLIP_TABLES: Table[] = ['clips'];

let seq = 4_000_000;
function changed() {
  seq += 1;
  void events.dbChanged.emit({ tables: CLIP_TABLES, seq });
}

function titleOf(meetingId: Generated.MeetingId): string {
  return fixtureMeetings.find((meeting) => meeting.id === meetingId)?.title ?? 'Meeting';
}

/** `name` as a safe file stem, like `deino-core::clips::naming`. */
function safeStem(name: string): string {
  const stem = name
    .replace(/[/\\:*?"<>|]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^\.+/, '');
  return stem === '' ? 'Clip' : stem.slice(0, 80);
}

function freePath(meetingId: Generated.MeetingId, name: string, except?: string): string {
  const folder = `/mock/recordings/${meetingId}/clips/`;
  const taken = new Set(
    clips.filter((clip) => clip.id !== except).map((clip) => clip.path.toLowerCase()),
  );
  const stem = safeStem(name);
  for (let n = 1; ; n += 1) {
    const path = `${folder}${n === 1 ? stem : `${stem} ${String(n)}`}.m4a`;
    if (!taken.has(path.toLowerCase())) return path;
  }
}

let clips: Clip[] = [
  {
    id: '0192a5f0-0000-7000-8000-00000000c001' as Generated.ClipId,
    meetingId: STANDUP,
    meetingTitle: titleOf(STANDUP),
    name: 'Release cut plan',
    start: 12_000,
    end: 27_500,
    media: 'audio',
    path: `/mock/recordings/${STANDUP}/clips/Release cut plan.m4a`,
    createdAt: SEEDED_AT,
  },
];

const invalid = (message: string) => failure<never>({ code: 'invalid', message });
const missing = () => failure<never>({ code: 'not_found', message: 'This clip no longer exists.' });

export const commands = {
  async create(clip) {
    await latency();
    if (clip.name.trim() === '') return invalid('Give the clip a name.');
    if (clip.end <= clip.start) return invalid('Choose a range to clip.');
    const id = crypto.randomUUID() as Generated.ClipId;
    const path = freePath(clip.meetingId, clip.name);
    const created: Clip = {
      ...clip,
      id,
      name: clip.name.trim(),
      meetingTitle: titleOf(clip.meetingId),
      media: 'audio',
      path,
      createdAt: new Date().toISOString(),
    };
    clips = [...clips, created];
    changed();
    return ok(created);
  },
  async list(meetingId) {
    await latency();
    return ok(
      clips.filter((clip) => clip.meetingId === meetingId).sort((a, b) => a.start - b.start),
    );
  },
  async listAll() {
    await latency();
    return ok([...clips].sort((a, b) => b.createdAt.localeCompare(a.createdAt)));
  },
  async rename(id, name) {
    await latency();
    const clip = clips.find((each) => each.id === id);
    if (clip === undefined) return missing();
    if (name.trim() === '') return invalid('Give the clip a name.');
    const renamed = { ...clip, name: name.trim(), path: freePath(clip.meetingId, name, id) };
    clips = clips.map((each) => (each.id === id ? renamed : each));
    changed();
    return ok(renamed);
  },
  async delete(id) {
    await latency();
    if (!clips.some((clip) => clip.id === id)) return missing();
    clips = clips.filter((clip) => clip.id !== id);
    changed();
    return ok(null);
  },
  async reveal(id) {
    await latency();
    return clips.some((clip) => clip.id === id) ? ok(null) : missing();
  },
} satisfies typeof Generated.commands;
