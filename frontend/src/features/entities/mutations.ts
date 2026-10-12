/**
 * TanStack Query mutations for people and companies. Each refetches the
 * entity queries when it succeeds, so lists, pages and attendee chips
 * update at once (`DbChanged` follows and reconciles).
 */
import { useMutation, useQueryClient } from '@tanstack/react-query';

import { companiesApi, peopleApi } from './api';
import { entityKeys } from './queries';
import type { CompanyFields, CompanyId, MeetingId, PersonFields, PersonId } from './types';

/** Refetches every entity query. */
function useInvalidateEntities() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: entityKeys.all });
}

/** Adds a person. */
export function useCreatePerson() {
  const onSuccess = useInvalidateEntities();
  return useMutation({ mutationFn: (fields: PersonFields) => peopleApi.create(fields), onSuccess });
}

/** Saves the person form. */
export function useUpdatePerson() {
  const onSuccess = useInvalidateEntities();
  return useMutation({
    mutationFn: ({ id, fields }: { id: PersonId; fields: PersonFields }) =>
      peopleApi.update(id, fields),
    onSuccess,
  });
}

/** Saves the user's notes about a person. */
export function useSetPersonNotes() {
  return useMutation({
    mutationFn: ({ id, notes }: { id: PersonId; notes: string }) => peopleApi.setNotes(id, notes),
  });
}

/** Merges one person into another. */
export function useMergePeople() {
  const onSuccess = useInvalidateEntities();
  return useMutation({
    mutationFn: ({ survivor, absorbed }: { survivor: PersonId; absorbed: PersonId }) =>
      peopleApi.merge(survivor, absorbed),
    onSuccess,
  });
}

/** Rewrites a person's AI summary now. */
export function useRefreshPersonSummary() {
  const onSuccess = useInvalidateEntities();
  return useMutation({ mutationFn: (id: PersonId) => peopleApi.refreshSummary(id), onSuccess });
}

/** Adds a company. */
export function useCreateCompany() {
  const onSuccess = useInvalidateEntities();
  return useMutation({
    mutationFn: (fields: CompanyFields) => companiesApi.create(fields),
    onSuccess,
  });
}

/** Saves the company form. */
export function useUpdateCompany() {
  const onSuccess = useInvalidateEntities();
  return useMutation({
    mutationFn: ({ id, fields }: { id: CompanyId; fields: CompanyFields }) =>
      companiesApi.update(id, fields),
    onSuccess,
  });
}

/** Saves the user's notes about a company. */
export function useSetCompanyNotes() {
  return useMutation({
    mutationFn: ({ id, notes }: { id: CompanyId; notes: string }) =>
      companiesApi.setNotes(id, notes),
  });
}

/** Merges one company into another. */
export function useMergeCompanies() {
  const onSuccess = useInvalidateEntities();
  return useMutation({
    mutationFn: ({ survivor, absorbed }: { survivor: CompanyId; absorbed: CompanyId }) =>
      companiesApi.merge(survivor, absorbed),
    onSuccess,
  });
}

/** Rewrites a company's AI summary now. */
export function useRefreshCompanySummary() {
  const onSuccess = useInvalidateEntities();
  return useMutation({
    mutationFn: (id: CompanyId) => companiesApi.refreshSummary(id),
    onSuccess,
  });
}

/** Fetches a company's logo (again, with `refresh`). */
export function useFetchLogo() {
  const onSuccess = useInvalidateEntities();
  return useMutation({
    mutationFn: ({ id, refresh }: { id: CompanyId; refresh: boolean }) =>
      companiesApi.fetchLogo(id, refresh),
    onSuccess,
  });
}

/** Adds or removes someone on a meeting. */
export function useAttendance(meetingId: MeetingId) {
  const onSuccess = useInvalidateEntities();
  const add = useMutation({
    mutationFn: (personId: PersonId) => peopleApi.addAttendee(meetingId, personId),
    onSuccess,
  });
  const remove = useMutation({
    mutationFn: (personId: PersonId) => peopleApi.removeAttendee(meetingId, personId),
    onSuccess,
  });
  return { add, remove };
}
