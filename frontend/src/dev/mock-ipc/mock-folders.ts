/**
 * Folders for `pnpm dev:mock`: an in-memory twin of `deino_core::folders`
 * with its rules (sibling names unique ignoring case, no cycles, deleting
 * keeps meetings, the series offer), reporting writes as `DbChanged` tables.
 */
import type * as Generated from '@/lib/ipc/generated/folders';
import type { AppError, Table } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';

type Folder = Generated.Folder;
type FolderId = Generated.FolderId;
type MeetingId = Generated.MeetingId;
type Meeting = Generated.Meeting;
type SeriesId = Generated.SeriesId;
type Rule = Generated.FolderRuleEntry;

/** What the folders reach outside themselves. */
export interface MockFolderDeps {
  /** Every meeting, newest first. */
  meetings: () => Meeting[];
  /** The meeting's recurring series. */
  seriesOf: (meeting: MeetingId) => SeriesId | null;
  onChange: (tables: Table[]) => void;
}

/** The folder state the mock starts from. */
export interface MockFolderSeed {
  folders: Folder[];
  filings: { meetingId: MeetingId; folderId: FolderId }[];
  rules: Rule[];
}

function notFound(id: string): AppError {
  return { code: 'not_found', message: `folder ${id} was not found` };
}

/** The folder tree, filings and rules. */
export class MockFolders {
  private state: MockFolderSeed;

  constructor(
    seed: MockFolderSeed,
    private readonly deps: MockFolderDeps,
  ) {
    this.state = seed;
  }

  tree(): Generated.FolderTree {
    const entries: Generated.FolderTreeEntry[] = [];
    const walk = (parent: FolderId | null, depth: number) => {
      for (const folder of this.childrenOf(parent)) {
        entries.push({
          folder: { ...folder },
          depth,
          meetingCount: this.meetingsIn(folder.id).length,
        });
        walk(folder.id, depth + 1);
      }
    };
    walk(null, 0);
    return { entries, unfiledCount: this.unfiled().length };
  }

  get(id: FolderId): CommandResult<Folder> {
    const folder = this.find(id);
    return folder === undefined ? failure(notFound(id)) : ok({ ...folder });
  }

  create(input: Generated.NewFolder, now: Date): CommandResult<Folder> {
    const name = input.name.trim();
    const problem = this.nameProblem(name, input.parentId, null);
    if (problem !== null) return failure(problem);
    const stamp = now.toISOString();
    const folder: Folder = {
      id: crypto.randomUUID() as FolderId,
      parentId: input.parentId,
      name,
      icon: null,
      description: '',
      createdAt: stamp,
      updatedAt: stamp,
    };
    this.state.folders.push(folder);
    this.deps.onChange(['folders']);
    return ok({ ...folder });
  }

  /** Applies `change` to the folder after `check` passes. */
  update(id: FolderId, change: Partial<Folder>, now: Date): CommandResult<Folder> {
    const folder = this.find(id);
    if (folder === undefined) return failure(notFound(id));
    Object.assign(folder, change, { updatedAt: now.toISOString() });
    this.deps.onChange(['folders']);
    return ok({ ...folder });
  }

  rename(id: FolderId, input: string, now: Date): CommandResult<Folder> {
    const name = input.trim();
    const problem = this.nameProblem(name, this.find(id)?.parentId ?? null, id);
    return problem === null ? this.update(id, { name }, now) : failure(problem);
  }

  moveTo(id: FolderId, parentId: FolderId | null, now: Date): CommandResult<Folder> {
    const folder = this.find(id);
    if (folder === undefined) return failure(notFound(id));
    if (parentId !== null && this.subtree(id).has(parentId)) {
      return failure({
        code: 'conflict',
        message: 'a folder cannot move into itself or one of its subfolders',
      });
    }
    const problem = this.nameProblem(folder.name, parentId, id);
    return problem === null ? this.update(id, { parentId }, now) : failure(problem);
  }

  delete(id: FolderId): CommandResult<null> {
    if (this.find(id) === undefined) return failure(notFound(id));
    const doomed = this.subtree(id);
    this.state.folders = this.state.folders.filter((folder) => !doomed.has(folder.id));
    this.state.filings = this.state.filings.filter((filing) => !doomed.has(filing.folderId));
    this.state.rules = this.state.rules.filter((rule) => !doomed.has(rule.folderId));
    this.deps.onChange(['folders', 'meeting_folders', 'folder_rules']);
    return ok(null);
  }

  meetingsIn(id: FolderId): Meeting[] {
    const inside = new Set(
      this.state.filings.filter((f) => f.folderId === id).map((f) => f.meetingId),
    );
    return this.deps.meetings().filter((meeting) => inside.has(meeting.id));
  }

  unfiled(): Meeting[] {
    const filed = new Set(this.state.filings.map((filing) => filing.meetingId));
    return this.deps.meetings().filter((meeting) => !filed.has(meeting.id));
  }

  foldersOf(meetingId: MeetingId): Folder[] {
    const holding = new Set(
      this.state.filings.filter((f) => f.meetingId === meetingId).map((f) => f.folderId),
    );
    return this.byName(this.state.folders.filter((folder) => holding.has(folder.id)));
  }

  addMeeting(folderId: FolderId, meetingId: MeetingId): CommandResult<Generated.FiledMeeting> {
    if (this.find(folderId) === undefined) return failure(notFound(folderId));
    if (!this.state.filings.some((f) => f.folderId === folderId && f.meetingId === meetingId)) {
      this.state.filings.push({ folderId, meetingId });
      this.deps.onChange(['meeting_folders']);
    }
    const series = this.deps.seriesOf(meetingId);
    const collected = this.state.rules.some(
      (rule) =>
        rule.folderId === folderId && rule.rule.kind === 'series' && rule.rule.seriesId === series,
    );
    return ok({ seriesOffer: series !== null && !collected ? series : null });
  }

  removeMeeting(folderId: FolderId, meetingId: MeetingId): CommandResult<Generated.UnfiledMeeting> {
    if (this.find(folderId) === undefined) return failure(notFound(folderId));
    this.state.filings = this.state.filings.filter(
      (f) => !(f.folderId === folderId && f.meetingId === meetingId),
    );
    const series = this.deps.seriesOf(meetingId);
    const before = this.state.rules.length;
    this.state.rules = this.state.rules.filter(
      (rule) =>
        !(
          rule.folderId === folderId &&
          rule.rule.kind === 'series' &&
          rule.rule.seriesId === series
        ),
    );
    this.deps.onChange(['meeting_folders', 'folder_rules']);
    return ok({ seriesRuleRemoved: this.state.rules.length < before });
  }

  /** The folder's rules; a series rule names its latest meeting, like the SQL. */
  rulesOf(folderId: FolderId): Rule[] {
    return this.state.rules
      .filter((rule) => rule.folderId === folderId)
      .map((rule) => ({ ...rule, seriesTitle: this.seriesTitle(rule.rule) }));
  }

  private seriesTitle(rule: Generated.FolderRule): string | null {
    if (rule.kind !== 'series') return null;
    const latest = this.deps.meetings().find((m) => this.deps.seriesOf(m.id) === rule.seriesId);
    return latest?.title ?? null;
  }

  /** Stores `rule` unless the folder already has it; returns the stored rule. */
  addRule(folderId: FolderId, rule: Generated.FolderRule, now: Date): CommandResult<Rule> {
    if (this.find(folderId) === undefined) return failure(notFound(folderId));
    const same = (stored: Rule) =>
      stored.folderId === folderId && JSON.stringify(stored.rule) === JSON.stringify(rule);
    const existing = this.state.rules.find(same);
    if (existing !== undefined) return ok({ ...existing });
    const entry: Rule = {
      id: crypto.randomUUID() as Rule['id'],
      folderId,
      rule,
      createdAt: now.toISOString(),
      seriesTitle: null,
    };
    this.state.rules.push(entry);
    this.deps.onChange(['folder_rules']);
    return ok({ ...entry });
  }

  removeRule(id: Rule['id']): CommandResult<null> {
    if (!this.state.rules.some((rule) => rule.id === id))
      return failure({ code: 'not_found', message: `rule ${id} was not found` });
    this.state.rules = this.state.rules.filter((rule) => rule.id !== id);
    this.deps.onChange(['folder_rules']);
    return ok(null);
  }

  private find(id: FolderId): Folder | undefined {
    return this.state.folders.find((folder) => folder.id === id);
  }

  private byName(folders: Folder[]): Folder[] {
    return [...folders].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
  }

  private childrenOf(parent: FolderId | null): Folder[] {
    return this.byName(this.state.folders.filter((folder) => folder.parentId === parent));
  }

  private subtree(id: FolderId): Set<FolderId> {
    const ids = new Set<FolderId>([id]);
    for (const folder of this.childrenOf(id))
      for (const child of this.subtree(folder.id)) ids.add(child);
    return ids;
  }

  private nameProblem(
    name: string,
    parent: FolderId | null,
    except: FolderId | null,
  ): AppError | null {
    if (name === '') return { code: 'invalid', message: 'a folder name cannot be empty' };
    const taken = this.childrenOf(parent).some(
      (sibling) => sibling.id !== except && sibling.name.toLowerCase() === name.toLowerCase(),
    );
    return taken
      ? { code: 'conflict', message: 'a folder with that name already exists here' }
      : null;
  }
}
