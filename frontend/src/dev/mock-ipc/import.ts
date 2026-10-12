/**
 * Mock of `@/lib/ipc/generated/import`: the dialogs choose a fixed video
 * and folder, an import "finishes" as the one-on-one fixture meeting (so the
 * event chip has a meeting to link), and Settings → Watched folders starts
 * with one folder and one failed file.
 */
import type * as Generated from '@/lib/ipc/generated/import';
import type { Table } from '@/lib/ipc/generated/shared';

import { failure, ok } from './command-result';
import { events } from './db';
import { latency } from './latency';

const ONE_ON_ONE = 'meeting-fixture-one-on-one' as Generated.MeetingId;
const SEEDED_AT = '2026-10-08T09:00:00.000Z';
const INBOX = '/Users/me/Movies/Zoom';

let seq = 4_000_000;
function changed(tables: Table[]) {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
}

function folder(id: string, path: string, moveToProcessed: boolean): Generated.WatchedFolder {
  return { id: id as Generated.WatchedFolderId, path, moveToProcessed, createdAt: SEEDED_AT };
}

let folders: Generated.WatchedFolder[] = [
  folder('0192a5f0-0000-7000-8000-00000000c001', INBOX, true),
];

let failures: Generated.WatchFailure[] = [
  {
    id: '0192a5f0-0000-7000-8000-00000000c101' as Generated.WatchedFileId,
    folderId: '0192a5f0-0000-7000-8000-00000000c001' as Generated.WatchedFolderId,
    path: `${INBOX}/Screen Recording 2026-10-07.mov`,
    failure: 'no_audio',
    failedAt: SEEDED_AT,
  },
];

/** The file a path names, as the dialog shows it, or why it is refused. */
function candidate(path: string) {
  const fileName = path.slice(path.lastIndexOf('/') + 1);
  const extension = fileName.slice(fileName.lastIndexOf('.') + 1).toLowerCase();
  const video = ['mp4', 'mov', 'm4v', 'mkv', 'webm'].includes(extension);
  const audio = ['m4a', 'mp3', 'wav', 'aac', 'aif', 'aiff', 'caf', 'flac', 'ogg', 'oga', 'opus'];
  if (!video && !audio.includes(extension)) {
    return failure<Generated.ImportCandidate>({
      code: 'invalid',
      message: 'this kind of file cannot be imported',
    });
  }
  const kind: Generated.MediaKind = video ? 'video' : 'audio';
  return ok({ path, fileName, kind });
}

export const commands = {
  async pickFile() {
    await latency();
    return candidate('/Users/me/Downloads/Customer call.mp4');
  },
  async check(path) {
    await latency();
    return candidate(path);
  },
  async start(path, title) {
    // Converting takes a moment, as it does for real.
    await new Promise((resolve) => setTimeout(resolve, 1_200));
    const fileName = path.slice(path.lastIndexOf('/') + 1);
    const named = title ?? fileName.slice(0, fileName.lastIndexOf('.'));
    changed(['meetings']);
    return ok({ meetingId: ONE_ON_ONE, title: named, duration: 3_312, hasVideo: true });
  },
  async watchedFolders() {
    await latency();
    return ok(folders);
  },
  async addWatchedFolder() {
    await latency();
    const path = '/Users/me/Documents/Voice Memos';
    if (folders.some((f) => f.path === path)) {
      return failure<Generated.WatchedFolder | null>({
        code: 'conflict',
        message: 'this folder is already watched',
      });
    }
    const added = folder(crypto.randomUUID(), path, false);
    folders = [...folders, added];
    changed(['watched_folders']);
    return ok(added);
  },
  async removeWatchedFolder(id) {
    await latency();
    folders = folders.filter((f) => f.id !== id);
    failures = failures.filter((f) => f.folderId !== id);
    changed(['watched_folders', 'watched_files']);
    return ok(null);
  },
  async setMoveToProcessed(id, on) {
    await latency();
    folders = folders.map((f) => (f.id === id ? { ...f, moveToProcessed: on } : f));
    changed(['watched_folders']);
    return ok(null);
  },
  async watchFailures() {
    await latency();
    return ok(failures);
  },
  async retryFailure(id) {
    await latency();
    failures = failures.filter((f) => f.id !== id);
    changed(['watched_files']);
    return ok(null);
  },
  async dismissFailure(id) {
    await latency();
    failures = failures.filter((f) => f.id !== id);
    changed(['watched_files']);
    return ok(null);
  },
} satisfies typeof Generated.commands;
