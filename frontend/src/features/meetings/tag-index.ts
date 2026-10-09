/**
 * Which tags each meeting carries, for the Meetings page's tag marks and
 * its tag filter: one pass over the tag links (ORG-06).
 */
import { type Tag, type TagId, useTagLinks, useTags } from '@/features/tags';

import type { Meeting, MeetingId } from './types';

/** A meeting's tags, by meeting. */
export type TagIndex = ReadonlyMap<MeetingId, readonly Tag[]>;

/** Builds the index from the tags and their links (by name, as the list shows them). */
export function buildTagIndex(
  tags: readonly Tag[],
  links: readonly { meetingId: MeetingId; tagId: TagId }[],
): TagIndex {
  const byId = new Map(tags.map((tag) => [tag.id, tag]));
  const index = new Map<MeetingId, Tag[]>();
  for (const link of links) {
    const tag = byId.get(link.tagId);
    if (tag === undefined) continue;
    const onMeeting = index.get(link.meetingId) ?? [];
    onMeeting.push(tag);
    index.set(link.meetingId, onMeeting);
  }
  for (const onMeeting of index.values()) onMeeting.sort((a, b) => a.name.localeCompare(b.name));
  return index;
}

/** The meetings carrying any of `filter`; all of them for an empty filter. */
export function filterByTags(
  meetings: readonly Meeting[],
  index: TagIndex,
  filter: readonly TagId[],
): readonly Meeting[] {
  if (filter.length === 0) return meetings;
  return meetings.filter((meeting) =>
    (index.get(meeting.id) ?? []).some((tag) => filter.includes(tag.id)),
  );
}

/** The live tag index, and whether the links have loaded (a filter waits for them). */
export function useTagIndex(): { index: TagIndex; ready: boolean } {
  const tags = useTags();
  const links = useTagLinks();
  return {
    index: buildTagIndex(tags.data ?? [], links.data ?? []),
    ready: tags.isSuccess && links.isSuccess,
  };
}
