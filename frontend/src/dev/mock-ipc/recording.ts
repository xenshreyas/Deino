/**
 * Mock of `@/lib/ipc/generated/recording`: a recorder that "hears" a short
 * scripted conversation, so `pnpm dev:mock` shows the record screen, the
 * sidebar's live row and the live transcript. Start creates a meeting in the
 * mock database; every couple of seconds a Me or Them line grows word by
 * word, then is saved; stop returns the meeting.
 */
import type * as Generated from '@/lib/ipc/generated/recording';

import { failure, ok } from './command-result';
import { mockDatabase, mockEventLinks, mockTranscripts } from './database';
import { latency } from './latency';
import { mockEvent } from './mock-event';
import { panelCommands, panelEvents } from './mock-record-panel';

/** How often a word is heard. */
const WORD_MS = 350;

/** The conversation, as (channel, line) pairs; it loops. */
const SCRIPT: readonly (readonly [Generated.Channel, string])[] = [
  ['system', 'Thanks for joining, shall we start with the launch plan?'],
  ['mic', 'Sure, the beta build is ready for the first testers on Monday.'],
  ['system', 'Great. What is still open on the onboarding flow?'],
  ['mic', 'Only the permission screen copy, which I will finish tomorrow.'],
];

const IDLE: Generated.RecordingSnapshot = {
  phase: 'idle',
  meetingId: null,
  title: null,
  startedAt: null,
  pausedMs: 0,
  pausedAt: null,
  echo: null,
  calendarEventId: null,
  microphone: null,
  systemAudio: true,
  issues: [],
};

export const events = {
  ...panelEvents,
  health: mockEvent<Generated.Health>('plugin:recording:health'),
  liveSegment: mockEvent<Generated.LiveSegment>('plugin:recording:live-segment'),
  recordingState: mockEvent<Generated.RecordingState>('plugin:recording:recording-state'),
} satisfies typeof Generated.events;

let snapshot: Generated.RecordingSnapshot = IDLE;
let timer: number | undefined;
let heard = 0;

function publish(next: Generated.RecordingSnapshot): Generated.RecordingSnapshot {
  snapshot = next;
  void events.recordingState.emit({ snapshot: next });
  return next;
}

function elapsedMs(now: number): number {
  if (snapshot.startedAt === null) return 0;
  return now - Date.parse(snapshot.startedAt) - snapshot.pausedMs;
}

/** The line being spoken after `words` words in total. */
function lineAt(words: number): { index: number; words: number; total: number } {
  let rest = words;
  for (let index = 0; ; index += 1) {
    const total = (SCRIPT[index % SCRIPT.length]?.[1] ?? '').split(' ').length;
    if (rest < total) return { index, words: rest + 1, total };
    rest -= total;
  }
}

/** One heard word: the open line grows; its last word saves it. */
function hear(meetingId: Generated.MeetingId): void {
  if (snapshot.phase !== 'recording') return;
  const line = lineAt(heard);
  heard += 1;
  const [channel, text] = SCRIPT[line.index % SCRIPT.length] ?? ['mic', ''];
  const done = line.words === line.total;
  const start = Math.max(0, elapsedMs(Date.now()) - line.words * WORD_MS);
  const update: Generated.SegmentUpdate = {
    meetingId,
    segmentId: `segment-mock-${String(line.index)}` as Generated.SegmentId,
    channel,
    start,
    end: done ? start + line.total * WORD_MS : start,
    text: text.split(' ').slice(0, line.words).join(' '),
    state: done ? 'final' : 'open',
  };
  void events.liveSegment.emit({ update });
  if (done) save(update);
}

/** Saves a finished line (with no speaker: the view names it by channel). */
function save(update: Generated.SegmentUpdate): void {
  mockTranscripts.save({
    id: update.segmentId,
    meetingId: update.meetingId,
    channel: update.channel,
    speakerId: null,
    start: update.start,
    end: update.end,
    text: update.text,
    words: [],
    isDeleted: false,
    editedAt: null,
    originalText: null,
  });
  mockDatabase.touch(['transcript_segments']);
}

const conflict = (message: string) => failure<never>({ code: 'conflict', message });

async function start(title: string | null) {
  await latency();
  if (snapshot.phase !== 'idle') return conflict('a recording is already in progress');
  const created = mockDatabase.addScheduled(title ?? 'Untitled meeting', new Date());
  if (created.status === 'error') return created;
  const meetingId = created.data.id;
  // Rust links the nearest calendar event once the meeting exists (ORG-03).
  const linkedTitle = mockEventLinks.autoLink(meetingId, new Date(), title !== null);
  const base = { ...IDLE, meetingId, title: linkedTitle ?? created.data.title };
  publish({ ...base, phase: 'starting' });
  publish({ ...base, phase: 'recording', startedAt: new Date().toISOString(), echo: 'cancel' });
  heard = 0;
  timer = window.setInterval(() => {
    hear(meetingId);
  }, WORD_MS);
  return ok(meetingId);
}

async function stop() {
  await latency();
  const meetingId = snapshot.meetingId;
  if (meetingId === null || snapshot.phase === 'stopping') {
    return conflict('nothing is being recorded');
  }
  window.clearInterval(timer);
  publish({ ...snapshot, phase: 'stopping' });
  publish(IDLE);
  return ok(meetingId);
}

async function pause() {
  await latency();
  if (snapshot.phase !== 'recording') return conflict('nothing is being recorded');
  return ok(publish({ ...snapshot, phase: 'paused', pausedAt: new Date().toISOString() }));
}

async function resume() {
  await latency();
  if (snapshot.phase !== 'paused' || snapshot.pausedAt === null) {
    return conflict('nothing is being recorded');
  }
  const pausedMs = snapshot.pausedMs + Date.now() - Date.parse(snapshot.pausedAt);
  return ok(publish({ ...snapshot, phase: 'recording', pausedAt: null, pausedMs }));
}

export const commands = {
  start,
  stop,
  pause,
  resume,
  async state() {
    await latency();
    return ok(snapshot);
  },
  async resumeMeeting() {
    await latency();
    return failure<Generated.MeetingId>({
      code: 'unavailable',
      message: 'resuming a meeting needs the app',
    });
  },
  ...panelCommands({ get: () => snapshot, publish }),
} satisfies typeof Generated.commands;
