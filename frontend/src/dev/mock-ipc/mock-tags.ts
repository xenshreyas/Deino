/**
 * Tags for `pnpm dev:mock`: an in-memory twin of `deino_core::tags` with
 * its rules (names unique ignoring case, the first applier kept, the next
 * palette colour), reporting writes as `DbChanged` tables.
 */
import type * as Generated from '@/lib/ipc/generated/tags';
import type { AppError, Table } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';

type Tag = Generated.Tag;
type TagId = Generated.TagId;
type MeetingId = Generated.MeetingId;
type Link = Generated.MeetingTagLink;

/** Same limit as `deino_core::tags::MAX_TAG_NAME_CHARS`. */
const MAX_NAME_CHARS = 40;

function invalid(message: string): AppError {
  return { code: 'invalid', message };
}

/** The tags and their links, with change notifications. */
export class MockTags {
  constructor(
    private tags: Tag[],
    private links: Link[],
    private readonly onChange: (tables: Table[]) => void,
  ) {}

  list(): Tag[] {
    return [...this.tags].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
  }

  create(input: Generated.NewTag, now: Date): CommandResult<Tag> {
    const name = input.name.trim();
    const problem = this.nameProblem(name, null);
    if (problem !== null) return failure(problem);
    const tag: Tag = {
      id: crypto.randomUUID() as TagId,
      name,
      color: input.color ?? (this.tags.length % 6) + 1,
      description: input.description.trim(),
      autoApply: true,
      createdAt: now.toISOString(),
      updatedAt: now.toISOString(),
    };
    this.tags.push(tag);
    this.onChange(['tags']);
    return ok({ ...tag });
  }

  update(id: TagId, update: Generated.TagUpdate, now: Date): CommandResult<Tag> {
    const index = this.tags.findIndex((tag) => tag.id === id);
    const current = this.tags[index];
    if (current === undefined) return failure(notFound(id));
    const name = update.name.trim();
    const problem = this.nameProblem(name, id);
    if (problem !== null) return failure(problem);
    const tag = {
      ...current,
      ...update,
      name,
      description: update.description.trim(),
      updatedAt: now.toISOString(),
    };
    this.tags[index] = tag;
    this.onChange(['tags']);
    return ok({ ...tag });
  }

  delete(id: TagId): CommandResult<null> {
    if (!this.tags.some((tag) => tag.id === id)) return failure(notFound(id));
    this.tags = this.tags.filter((tag) => tag.id !== id);
    this.links = this.links.filter((link) => link.tagId !== id);
    this.onChange(['tags', 'meeting_tags', 'folder_rules']);
    return ok(null);
  }

  tagsOf(meetingId: MeetingId): Generated.AppliedTag[] {
    return this.list().flatMap((tag) => {
      const link = this.links.find(
        (candidate) => candidate.meetingId === meetingId && candidate.tagId === tag.id,
      );
      return link === undefined
        ? []
        : [{ tag, appliedBy: link.appliedBy, appliedAt: '2026-10-09T09:00:00.000Z' }];
    });
  }

  allLinks(): Link[] {
    return this.links.map((link) => ({ ...link }));
  }

  apply(meetingId: MeetingId, tagId: TagId): CommandResult<null> {
    if (!this.tags.some((tag) => tag.id === tagId)) return failure(notFound(tagId));
    if (this.links.some((link) => link.meetingId === meetingId && link.tagId === tagId))
      return ok(null);
    this.links.push({ meetingId, tagId, appliedBy: 'user' });
    this.onChange(['meeting_tags']);
    return ok(null);
  }

  remove(meetingId: MeetingId, tagId: TagId): CommandResult<null> {
    this.links = this.links.filter(
      (link) => !(link.meetingId === meetingId && link.tagId === tagId),
    );
    this.onChange(['meeting_tags']);
    return ok(null);
  }

  private nameProblem(name: string, except: TagId | null): AppError | null {
    if (name === '') return invalid('a tag name cannot be empty');
    if (name.length > MAX_NAME_CHARS)
      return invalid(`a tag name can be at most ${MAX_NAME_CHARS} characters long`);
    const taken = this.tags.some(
      (tag) => tag.id !== except && tag.name.toLowerCase() === name.toLowerCase(),
    );
    return taken ? { code: 'conflict', message: 'a tag with that name already exists' } : null;
  }
}

function notFound(id: string): AppError {
  return { code: 'not_found', message: `tag ${id} was not found` };
}
