/**
 * The mock meetings' calendar links (`meetings.calendar_event_id`,
 * `series_id` and the attendees a link brings), behind the `calendar`
 * mock's linking commands. Every fixture meeting starts linked to its own
 * event, as the Rust seeder links it. Linking renames the meeting after the
 * event and swaps the invitees; the enhanced-notes mock reads `change` to
 * offer the regenerate prompt, like the Rust basis does.
 */
import type * as Calendar from '@/lib/ipc/generated/calendar';
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';
import {
  nearestEvent,
  relate,
  type RelationSubject,
  sameSet,
  suggestionsAround,
} from './mock-link-rules';
import type { MockDatabase } from './mock-database';

type CalendarEvent = Calendar.CalendarEvent;

/** `deino_testkit::fixture::USER_EMAIL`: the fixtures' "me". */
const USER_EMAIL = 'shreyas@deino.dev';

/** When a meeting's link last changed, and whether its attendees did. */
export interface LinkChange {
  at: string;
  attendeesChanged: boolean;
}

/** One meeting's link. */
interface Link {
  eventId: Calendar.CalendarEventId | null;
  seriesId: string | null;
  /** The attendees' emails. */
  attendees: ReadonlySet<string>;
  changed: LinkChange | null;
}

const UNLINKED: Link = { eventId: null, seriesId: null, attendees: new Set(), changed: null };

/** The invitees who did not decline, by email. */
function inviteesOf(event: CalendarEvent | null): Set<string> {
  const emails = (event?.attendees ?? [])
    .filter((guest) => guest.rsvp !== 'declined')
    .flatMap((guest) => (guest.email === null ? [] : [guest.email]));
  return new Set(emails);
}

/** How many of `next` are new, and how many of `previous` left. */
function counted(previous: ReadonlySet<string>, next: ReadonlySet<string>) {
  return {
    attendeesAdded: [...next].filter((email) => !previous.has(email)).length,
    attendeesRemoved: [...previous].filter((email) => !next.has(email)).length,
  };
}

/** The meetings' links, with ORG-03's rules. */
export class MockEventLinks {
  private links = new Map<string, Link>();

  constructor(
    private readonly fixtures: readonly FixtureMeeting[],
    private readonly events: readonly CalendarEvent[],
    private readonly database: MockDatabase,
  ) {
    this.reseed();
  }

  /** Every fixture meeting linked to its own event again. */
  reseed(): void {
    this.links = new Map(
      this.fixtures.map((fixture) => {
        const event = this.event(fixtureUuid(`event/${fixture.id}`));
        const link: Link = {
          eventId: event?.id ?? null,
          seriesId: event?.seriesId ?? null,
          attendees: inviteesOf(event),
          changed: null,
        };
        return [fixture.id, link];
      }),
    );
  }

  /** The event `meetingId` is linked to. */
  linked(meetingId: MeetingId): CalendarEvent | null {
    return this.event(this.link(meetingId).eventId);
  }

  /** The picker's suggestions around the meeting's start. */
  suggestions(meetingId: MeetingId): CalendarEvent[] {
    return suggestionsAround(this.events, Date.parse(this.happenedAt(meetingId)));
  }

  /** When the link last changed; `null` while it is the seeded one. */
  change(meetingId: MeetingId): LinkChange | null {
    return this.link(meetingId).changed;
  }

  /**
   * Links (or, for `null`, unlinks) the meeting, like `EventLinkService`;
   * `rename` titles it after the event.
   */
  apply(
    meetingId: MeetingId,
    eventId: Calendar.CalendarEventId | null,
    now: Date,
    rename = eventId !== null,
  ): CommandResult<Calendar.EventLinkChange> {
    const meeting = this.database.meeting(meetingId);
    if (meeting.status === 'error') return failure(meeting.error);
    const event = this.event(eventId);
    if (eventId !== null && event === null) {
      return failure({ code: 'not_found', message: `calendar event ${eventId} was not found` });
    }
    const previous = this.link(meetingId);
    const attendees = inviteesOf(event);
    this.links.set(meetingId, {
      eventId,
      seriesId: event?.seriesId ?? null,
      attendees,
      changed: { at: now.toISOString(), attendeesChanged: !sameSet(previous.attendees, attendees) },
    });
    this.database.touch(['meetings', 'meeting_attendees']);
    const title = event === null || !rename ? null : this.retitle(meetingId, event, now);
    return ok({ meetingId, event, title, ...counted(previous.attendees, attendees) });
  }

  /** Links a recording that just started to the nearest event; its new title. */
  autoLink(meetingId: MeetingId, now: Date, titled: boolean): string | null {
    const event = nearestEvent(this.events, now.getTime());
    if (event === null) return null;
    const applied = this.apply(meetingId, event.id, now, !titled);
    return applied.status === 'ok' ? applied.data.title : null;
  }

  /** The meetings related to `meetingId`. */
  related(meetingId: MeetingId): Calendar.RelatedMeetings {
    const subjects = this.database.meetings().map((meeting) => this.subject(meeting.id));
    return relate(this.subject(meetingId), subjects);
  }

  private retitle(meetingId: MeetingId, event: CalendarEvent, now: Date): string | null {
    const current = this.database.meeting(meetingId);
    if (current.status === 'ok' && current.data.title === event.title.trim()) return null;
    const renamed = this.database.rename(meetingId, event.title, now);
    return renamed.status === 'ok' ? renamed.data.title : null;
  }

  private subject(meetingId: MeetingId): RelationSubject {
    const link = this.link(meetingId);
    const meeting = this.database.meeting(meetingId);
    const people = new Set([...link.attendees].filter((email) => email !== USER_EMAIL));
    return {
      id: meetingId,
      title: meeting.status === 'ok' ? meeting.data.title : '',
      seriesId: link.seriesId,
      happenedAt: this.happenedAt(meetingId),
      people,
    };
  }

  /** When the meeting took place: its recording's start, else its creation. */
  private happenedAt(meetingId: MeetingId): string {
    const fixture = this.database.fixture(meetingId);
    const meeting = this.database.meeting(meetingId);
    const at = fixture?.startedAt ?? (meeting.status === 'ok' ? meeting.data.createdAt : '');
    return at === '' ? '' : new Date(at).toISOString();
  }

  private link(meetingId: string): Link {
    return this.links.get(meetingId) ?? UNLINKED;
  }

  private event(id: string | null): CalendarEvent | null {
    const event = this.events.find((candidate) => candidate.id === id);
    return event === undefined ? null : structuredClone(event);
  }
}
