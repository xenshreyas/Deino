/**
 * The fixture meeting schema (`fixtures/meetings/*.json`), validated with zod
 * at the boundary like any external data.
 *
 * It mirrors the Rust schema in `crates/deino-testkit/src/fixture/model.rs`:
 * strict objects (unknown keys fail, like `deny_unknown_fields`) and closed
 * enums. The Rust loader also checks cross-field rules; this side trusts them,
 * because `cargo test` fails first when a fixture breaks one.
 */
import { z } from 'zod';

const millis = z.number().int().nonnegative();
const email = z.string().regex(/^[^@\sA-Z]+@[^@\sA-Z]+\.[^@\sA-Z]+$/, 'a lower-case email address');
const isoTime = z.string().datetime();

/** The capture track a voice is heard on. */
export const channelSchema = z.enum(['mic', 'system']);

const calendarEventSchema = z
  .object({
    id: z.string(),
    icalUid: z.string(),
    title: z.string(),
    description: z.string(),
    startAt: isoTime,
    endAt: isoTime,
    platform: z.enum(['google_meet', 'zoom', 'teams', 'in_person']),
    conferenceUrl: z.string().url().nullable(),
    location: z.string().nullable(),
    organizerEmail: email,
    recurrence: z.enum(['none', 'daily', 'weekly']),
  })
  .strict();

const attendeeSchema = z
  .object({
    name: z.string(),
    email,
    title: z.string().nullable(),
    role: z.enum(['organizer', 'invitee']),
    rsvp: z.enum(['accepted', 'tentative', 'declined', 'needs_action']),
  })
  .strict();

const speakerSchema = z
  .object({ label: z.string(), attendee: email.nullable(), channel: channelSchema })
  .strict();

const wordSchema = z.object({ text: z.string(), startMs: millis, endMs: millis }).strict();

const segmentSchema = z
  .object({
    speaker: z.string(),
    channel: channelSchema,
    startMs: millis,
    endMs: millis,
    text: z.string().min(1),
    words: z.array(wordSchema),
  })
  .strict();

const evidence = z.string().min(1);

const groundTruthSchema = z
  .object({
    decisions: z.array(z.string()),
    actionItems: z.array(
      z
        .object({
          title: z.string(),
          assignee: email.nullable(),
          due: z.string().date().nullable(),
          evidence,
        })
        .strict(),
    ),
    facts: z.array(z.object({ question: z.string(), answer: z.string(), evidence }).strict()),
  })
  .strict();

const audioSchema = z.object({ mic: z.string(), system: z.string(), clipEndMs: millis }).strict();

/** One fixture meeting file. */
export const fixtureMeetingSchema = z
  .object({
    id: z.string().startsWith('meeting-fixture-'),
    title: z.string().min(1),
    kind: z.enum([
      'one_on_one',
      'standup',
      'sales_discovery',
      'interview',
      'board_update',
      'customer_support',
      'brainstorm',
      'planning',
    ]),
    startedAt: isoTime,
    durationMs: millis,
    calendarEvent: calendarEventSchema,
    companies: z.array(z.object({ name: z.string(), domain: z.string() }).strict()),
    attendees: z.array(attendeeSchema),
    speakers: z.array(speakerSchema),
    segments: z.array(segmentSchema),
    userNotes: z.string().nullable(),
    groundTruth: groundTruthSchema,
    audio: audioSchema.nullable(),
  })
  .strict();

/** A validated fixture meeting. */
export type FixtureMeeting = z.infer<typeof fixtureMeetingSchema>;
/** A validated transcript segment. */
export type FixtureSegment = z.infer<typeof segmentSchema>;
