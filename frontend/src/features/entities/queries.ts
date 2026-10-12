/**
 * TanStack Query reads for people and companies. Each names the tables it
 * reads, so `DbChanged` refreshes it after any write, including the
 * `update_entities` stage's summaries and a calendar sync's new people.
 * Mutations are in `mutations.ts`.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { companiesApi, peopleApi } from './api';
import type { CompanyId, MeetingId, PersonId } from './types';

/** What a list reads: the rows, their companies and the meetings counted. */
const LIST_TABLES = [
  'people',
  'companies',
  'meeting_attendees',
  'meetings',
] as const satisfies readonly Table[];

/** What a page reads: the list's tables, open items and upcoming events. */
const PAGE_TABLES = [
  ...LIST_TABLES,
  'action_items',
  'calendar_events',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `entityKeys.all`. */
export const entityKeys = {
  all: ['entities'] as const,
  people: () => [...entityKeys.all, 'people'] as const,
  person: (id: PersonId) => [...entityKeys.all, 'person', id] as const,
  companies: () => [...entityKeys.all, 'companies'] as const,
  company: (id: CompanyId) => [...entityKeys.all, 'company', id] as const,
  attendees: (meetingId: MeetingId) => [...entityKeys.all, 'attendees', meetingId] as const,
};

/** Everyone the user meets. Also usable from route loaders. */
export const peopleListQuery = () =>
  queryOptions({
    queryKey: entityKeys.people(),
    queryFn: peopleApi.list,
    meta: { tables: LIST_TABLES },
  });

/** One person page. Also usable from route loaders. */
export const personPageQuery = (id: PersonId) =>
  queryOptions({
    queryKey: entityKeys.person(id),
    queryFn: () => peopleApi.page(id),
    meta: { tables: PAGE_TABLES },
  });

/** Every company. Also usable from route loaders. */
export const companiesListQuery = () =>
  queryOptions({
    queryKey: entityKeys.companies(),
    queryFn: companiesApi.list,
    meta: { tables: LIST_TABLES },
  });

/** One company page. Also usable from route loaders. */
export const companyPageQuery = (id: CompanyId) =>
  queryOptions({
    queryKey: entityKeys.company(id),
    queryFn: () => companiesApi.page(id),
    meta: { tables: PAGE_TABLES },
  });

/** Everyone the user meets, most recently met first. */
export function usePeople() {
  return useQuery(peopleListQuery());
}

/** One person page. */
export function usePersonPage(id: PersonId) {
  return useQuery(personPageQuery(id));
}

/** Every company, most recently met first. */
export function useCompanies() {
  return useQuery(companiesListQuery());
}

/** One company page. */
export function useCompanyPage(id: CompanyId) {
  return useQuery(companyPageQuery(id));
}

/** A meeting's attendees, the organiser first and the user last. */
export function useMeetingAttendees(meetingId: MeetingId) {
  return useQuery({
    queryKey: entityKeys.attendees(meetingId),
    queryFn: () => peopleApi.attendees(meetingId),
    meta: { tables: ['meeting_attendees', 'people', 'companies'] },
  });
}
