/**
 * TanStack Query hooks for a meeting's calendar link and related meetings.
 *
 * The link lives on the meeting row and points at a synced event, and
 * related meetings follow titles, series and attendees, so the queries name
 * those tables and `DbChanged` keeps them fresh: a link from the picker, an
 * auto-link when recording starts, a sync that changes the event. Linking
 * also writes its answer straight into the cache, so the chip changes at
 * once.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { eventLinksApi } from './api';
import type { CalendarEventId, EventLinkChange, MeetingId } from './types';

/** What the linked event and the suggestions read. */
const LINK_TABLES = ['meetings', 'calendar_events'] as const satisfies readonly Table[];
/** What related meetings read. */
const RELATED_TABLES = ['meetings', 'meeting_attendees'] as const satisfies readonly Table[];

/** Query keys; every key starts with `eventLinkKeys.all`. */
export const eventLinkKeys = {
  all: ['event-links'] as const,
  linked: (id: MeetingId) => [...eventLinkKeys.all, 'linked', id] as const,
  suggestions: (id: MeetingId) => [...eventLinkKeys.all, 'suggestions', id] as const,
  related: (id: MeetingId) => [...eventLinkKeys.all, 'related', id] as const,
};

/** The event a meeting is linked to; `null` without one. */
export const linkedEventQuery = (id: MeetingId) =>
  queryOptions({
    queryKey: eventLinkKeys.linked(id),
    queryFn: () => eventLinksApi.linked(id),
    meta: { tables: LINK_TABLES },
  });

/** The picker's suggestions for a meeting. */
export const eventSuggestionsQuery = (id: MeetingId) =>
  queryOptions({
    queryKey: eventLinkKeys.suggestions(id),
    queryFn: () => eventLinksApi.suggestions(id),
    meta: { tables: LINK_TABLES },
  });

/** The meetings related to a meeting, with "Previous in series". */
export const relatedMeetingsQuery = (id: MeetingId) =>
  queryOptions({
    queryKey: eventLinkKeys.related(id),
    queryFn: () => eventLinksApi.related(id),
    meta: { tables: RELATED_TABLES },
  });

/** The event a meeting is linked to. */
export function useLinkedEvent(id: MeetingId) {
  return useQuery(linkedEventQuery(id));
}

/** The picker's suggestions, fetched only while the picker is open. */
export function useEventSuggestions(id: MeetingId, enabled: boolean) {
  return useQuery({ ...eventSuggestionsQuery(id), enabled });
}

/** The meetings related to a meeting. */
export function useRelatedMeetings(id: MeetingId) {
  return useQuery(relatedMeetingsQuery(id));
}

/** Input of {@link useLinkEvent}. */
export interface LinkEventInput {
  meetingId: MeetingId;
  /** The event to link, or `null` to unlink. */
  eventId: CalendarEventId | null;
}

/** Links a meeting to an event or unlinks it, and caches the new link. */
export function useLinkEvent() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ meetingId, eventId }: LinkEventInput): Promise<EventLinkChange> =>
      eventId === null ? eventLinksApi.unlink(meetingId) : eventLinksApi.link(meetingId, eventId),
    onSuccess: (change) => {
      client.setQueryData(eventLinkKeys.linked(change.meetingId), change.event);
    },
  });
}
