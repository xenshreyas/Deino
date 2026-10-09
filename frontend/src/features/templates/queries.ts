/**
 * TanStack Query hooks for templates.
 *
 * Queries name the tables they read in `meta.tables`, so `DbChanged`
 * refreshes them after any write. Mutations also write their result into
 * the cache for an instant update; the `DbChanged` that follows reconciles.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { MeetingId } from '@/features/meetings';
import type { Table } from '@/lib/ipc/generated/shared';

import { templatesApi } from './api';
import type { Template, TemplateChoice, TemplateContent, TemplateId } from './types';

/** The tables a template is stored in. */
const TEMPLATE_TABLES = ['templates', 'template_sections'] as const satisfies readonly Table[];
/** Everything a meeting's template choice depends on. */
const CHOICE_TABLES = [
  ...TEMPLATE_TABLES,
  'meetings',
  'meeting_attendees',
  'people',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `templateKeys.all`. */
export const templateKeys = {
  all: ['templates'] as const,
  list: () => [...templateKeys.all, 'list'] as const,
  detail: (id: TemplateId) => [...templateKeys.all, 'detail', id] as const,
  choice: (meeting: MeetingId) => [...templateKeys.all, 'choice', meeting] as const,
};

/** Every template, by name. */
export const templateListQuery = () =>
  queryOptions({
    queryKey: templateKeys.list(),
    queryFn: templatesApi.list,
    meta: { tables: TEMPLATE_TABLES },
  });

/** One template. */
export const templateQuery = (id: TemplateId) =>
  queryOptions({
    queryKey: templateKeys.detail(id),
    queryFn: () => templatesApi.get(id),
    meta: { tables: TEMPLATE_TABLES },
  });

/** The template a meeting gets, and why. */
export const meetingTemplateQuery = (meeting: MeetingId) =>
  queryOptions({
    queryKey: templateKeys.choice(meeting),
    queryFn: () => templatesApi.meetingTemplate(meeting),
    meta: { tables: CHOICE_TABLES },
  });

/** Every template, by name. */
export function useTemplates() {
  return useQuery(templateListQuery());
}

/** One template. */
export function useTemplate(id: TemplateId) {
  return useQuery(templateQuery(id));
}

/** The template a meeting gets, and why. */
export function useMeetingTemplate(meeting: MeetingId) {
  return useQuery(meetingTemplateQuery(meeting));
}

/** Writes a saved template into the cached list and its detail. */
function useCacheTemplate() {
  const client = useQueryClient();
  return (template: Template) => {
    client.setQueryData(templateKeys.detail(template.id), template);
    client.setQueryData(templateKeys.list(), (templates: Template[] | undefined) => {
      if (templates === undefined) return undefined;
      const others = templates.filter((candidate) => candidate.id !== template.id);
      return [...others, template].sort(byName);
    });
  };
}

/** Saves a new template, or the content of an existing user template. */
export function useSaveTemplate() {
  const cache = useCacheTemplate();
  return useMutation({
    mutationFn: ({ id, content }: { id: TemplateId | null; content: TemplateContent }) =>
      id === null ? templatesApi.create(content) : templatesApi.update(id, content),
    onSuccess: cache,
  });
}

/** Makes an editable copy of a template. */
export function useDuplicateTemplate() {
  const cache = useCacheTemplate();
  return useMutation({ mutationFn: templatesApi.duplicate, onSuccess: cache });
}

/** Imports a template file. */
export function useImportTemplate() {
  const cache = useCacheTemplate();
  return useMutation({ mutationFn: templatesApi.importJson, onSuccess: cache });
}

/** Deletes a user template. */
export function useDeleteTemplate() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: templatesApi.delete,
    onSuccess: (_nothing, id) => {
      client.setQueryData(templateKeys.list(), (templates: Template[] | undefined) =>
        templates?.filter((template) => template.id !== id),
      );
      client.removeQueries({ queryKey: templateKeys.detail(id) });
    },
  });
}

/** Renders a template against a meeting (no LLM call). */
export function usePreviewTemplate() {
  return useMutation({
    mutationFn: ({ content, meeting }: { content: TemplateContent; meeting: MeetingId }) =>
      templatesApi.preview(content, meeting),
  });
}

/** Chooses a meeting's template, or `null` for automatic selection. */
export function useSetMeetingTemplate(meeting: MeetingId) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (template: TemplateId | null) => templatesApi.setMeetingTemplate(meeting, template),
    onSuccess: (choice: TemplateChoice) => {
      client.setQueryData(templateKeys.choice(meeting), choice);
    },
  });
}

/** List order: by name ignoring case, as Rust sorts. */
export function byName(a: Template, b: Template): number {
  return a.content.name.localeCompare(b.content.name, 'en', { sensitivity: 'base' });
}
