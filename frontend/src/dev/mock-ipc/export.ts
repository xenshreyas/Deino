/**
 * Mock of `@/lib/ipc/generated/export`: previews rendered from the fixture
 * meetings (`export-render.ts`), saves that "write" the suggested file (or
 * many into `Downloads`), a
 * folder dialog that picks `~/Obsidian/Deino` (saved in the mock settings),
 * and an account export that
 * walks through the eight meetings with `FullExportProgress` events.
 */
import type * as Generated from '@/lib/ipc/generated/export';
import type { CommandResult } from '@/lib/ipc/result';

import { mockDatabase } from './database';
import * as render from './export-render';
import type { FixtureMeeting } from './fixture-schema';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { mockEvent } from './mock-event';
import { commands as settings } from './settings';

/** The folder the mock folder dialog answers with. */
const VAULT = '/Users/shreyas/Obsidian/Deino';
/** The folder the mock bulk export's dialog answers with (its name only). */
const BULK_FOLDER = 'Downloads';
/** How long the mock account export spends per meeting. */
const STEP_MS = 350;

let status: Generated.FullExportStatus = {
  state: 'idle',
  done: 0,
  total: 0,
  fileName: null,
  message: null,
};
let timer: ReturnType<typeof setTimeout> | undefined;

export const events = {
  fullExportProgress: mockEvent<Generated.FullExportProgress>('plugin:export:full-export-progress'),
} satisfies typeof Generated.events;

function notFound<T>(id: string): CommandResult<T> {
  return failure<T>({ code: 'not_found', message: `meeting ${id} was not found` });
}

function preview(
  meeting: FixtureMeeting,
  format: Generated.ExportFormat,
  sections: Generated.ExportSections,
): Generated.ExportPreview {
  const fileName = render.fileName(meeting, format);
  const textOf: Partial<Record<Generated.ExportFormat, () => string>> = {
    markdown: () => render.markdown(meeting, sections),
    text: () => render.text(meeting, sections),
    json: () => JSON.stringify({ format: 'deino.meeting', version: 1, meeting }, null, 2),
    srt: () => render.subtitles(meeting, false),
    vtt: () => render.subtitles(meeting, true),
  };
  const asText = textOf[format];
  return asText
    ? { format, kind: 'text', content: asText(), fileName }
    : { format, kind: 'html', content: render.html(meeting, sections), fileName };
}

function publish(next: Generated.FullExportStatus) {
  status = next;
  void events.fullExportProgress.emit({ status: { ...next } });
}

/** Advances the mock account export by one meeting. */
function step(done: number, total: number) {
  if (done >= total) {
    publish({ state: 'finished', done: total, total, fileName: 'Deino export.zip', message: null });
    return;
  }
  publish({ state: 'running', done: done + 1, total, fileName: null, message: null });
  timer = setTimeout(() => {
    step(done + 1, total);
  }, STEP_MS);
}

export const commands = {
  async preview(meetingId, format, sections) {
    await latency();
    const meeting = mockDatabase.fixture(meetingId);
    return meeting ? ok(preview(meeting, format, sections)) : notFound(meetingId);
  },
  async save(meetingId, format) {
    await latency();
    const meeting = mockDatabase.fixture(meetingId);
    return meeting ? ok({ fileName: render.fileName(meeting, format) }) : notFound(meetingId);
  },
  async saveMany(meetingIds) {
    await latency();
    const missing = meetingIds.find((id) => !mockDatabase.fixture(id));
    return missing === undefined
      ? ok({ count: meetingIds.length, folder: BULK_FOLDER })
      : notFound(missing);
  },
  async copy(meetingId) {
    await latency();
    return mockDatabase.fixture(meetingId) ? ok(null) : notFound(meetingId);
  },
  async email(meetingId) {
    await latency();
    return mockDatabase.fixture(meetingId) ? ok(null) : notFound(meetingId);
  },
  async chooseFolder() {
    // Like Rust: the dialog's answer is saved, and SettingsChanged follows.
    await settings.patch({ export: { folder: VAULT } });
    return ok(VAULT);
  },
  async startFullExport() {
    await latency();
    if (status.state === 'running') {
      return failure({
        code: 'conflict',
        message: 'an export of every meeting is already running',
      });
    }
    step(0, mockDatabase.meetings().length);
    return ok(true);
  },
  async cancelFullExport() {
    await latency();
    clearTimeout(timer);
    if (status.state === 'running') {
      publish({ state: 'cancelled', done: 0, total: 0, fileName: null, message: null });
    }
    return ok(null);
  },
  async fullExportStatus() {
    await latency();
    return ok({ ...status });
  },
} satisfies typeof Generated.commands;
