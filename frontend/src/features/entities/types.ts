/**
 * The people and companies feature's types. Shapes shared with Rust are
 * generated (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { CompanyId } from '@/lib/ipc/generated/companies';
import type { PersonId } from '@/lib/ipc/generated/people';

export type {
  Company,
  CompanyFields,
  CompanyPage,
  CompanyRow,
} from '@/lib/ipc/generated/companies';
export type {
  AttendeeRole,
  CompanyId,
  EntityActionItem,
  EntityMeeting,
  MeetingId,
  MeetingPerson,
  Person,
  PersonFields,
  PersonId,
  PersonPage,
  PersonRow,
  SummaryOutcome,
  UpcomingEvent,
} from '@/lib/ipc/generated/people';

/** How a list is ordered. */
export type EntitySort = 'recent' | 'name';

/** Brands a person ID that arrived as a plain string (a route param, a DOM attribute). */
export function asPersonId(raw: string): PersonId {
  if (raw.trim() === '') throw new TypeError('A person ID cannot be empty.');
  return raw as PersonId;
}

/** Brands a company ID that arrived as a plain string (a route param). */
export function asCompanyId(raw: string): CompanyId {
  if (raw.trim() === '') throw new TypeError('A company ID cannot be empty.');
  return raw as CompanyId;
}
