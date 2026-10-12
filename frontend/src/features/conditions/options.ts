/**
 * What a condition can name (templates, tags, folders, people, calendars),
 * read through their features, and how a condition of each kind starts.
 */
import { useQuery } from '@tanstack/react-query';

import { type CalendarId, calendarsQuery } from '@/features/calendar';
import { peopleListQuery, type PersonId } from '@/features/entities';
import { type FolderId, useFolderTree } from '@/features/folders';
import { type TagId, useTags } from '@/features/tags';
import { type TemplateId, useTemplates } from '@/features/templates';

import type { Condition, ConditionKind } from './types';

/** One thing a condition can point at. */
export interface ConditionOption<Id extends string = string> {
  id: Id;
  label: string;
}

/** Everything the pickers list, by what they name. */
export interface ConditionOptions {
  templates: ConditionOption<TemplateId>[];
  tags: ConditionOption<TagId>[];
  folders: ConditionOption<FolderId>[];
  people: ConditionOption<PersonId>[];
  calendars: ConditionOption<CalendarId>[];
}

/** The pickers' options; empty while they load. */
export function useConditionOptions(): ConditionOptions {
  const templates = useTemplates().data ?? [];
  const tags = useTags().data ?? [];
  const folders = useFolderTree().data?.entries ?? [];
  const people = useQuery(peopleListQuery()).data ?? [];
  const calendars = useQuery(calendarsQuery()).data ?? [];
  return {
    templates: templates.map((template) => ({ id: template.id, label: template.content.name })),
    tags: tags.map((tag) => ({ id: tag.id, label: tag.name })),
    folders: folders.map((entry) => ({
      id: entry.folder.id,
      label: `${' '.repeat(entry.depth)}${entry.folder.name}`,
    })),
    people: people.map((person) => ({ id: person.id, label: person.name })),
    calendars: calendars.map((calendar) => ({ id: calendar.id, label: calendar.title })),
  };
}

/** A starting condition of each kind that needs no option. */
export function blankCondition(kind: ConditionKind): Condition | null {
  switch (kind) {
    case 'title_contains':
    case 'title_lacks':
    case 'title_is':
    case 'title_is_not':
      return { kind, text: '' };
    case 'title_matches':
      return { kind, pattern: '' };
    case 'attendee_domain':
      return { kind, domain: '' };
    case 'external':
      return { kind, external: true };
    case 'domains_are_all':
    case 'domains_not_all':
      return { kind, domains: [] };
    case 'invitee_count':
      return { kind, comparison: 'equal', count: 2 };
    case 'duration_over':
      return { kind, minutes: 30 };
    default:
      return null;
  }
}

/** Which option list a pointing kind picks from. */
function listOf(kind: ConditionKind, options: ConditionOptions): ConditionOption[] | null {
  switch (kind) {
    case 'template_is':
      return options.templates;
    case 'has_tag':
    case 'lacks_tag':
      return options.tags;
    case 'in_folder':
      return options.folders;
    case 'has_participant':
    case 'lacks_participant':
      return options.people;
    case 'calendar_is':
      return options.calendars;
    default:
      return null;
  }
}

/** The ID a pointing condition names. */
function pointedId(condition: Condition): string | null {
  switch (condition.kind) {
    case 'template_is':
      return condition.templateId;
    case 'has_tag':
    case 'lacks_tag':
      return condition.tagId;
    case 'in_folder':
      return condition.folderId;
    case 'has_participant':
    case 'lacks_participant':
      return condition.personId;
    case 'calendar_is':
      return condition.calendarId;
    default:
      return null;
  }
}

/** The option with `id` in `list`, by its typed ID. */
function find<Id extends string>(list: ConditionOption<Id>[], id: string): Id | undefined {
  return list.find((option) => option.id === id)?.id;
}

/** A condition of `kind` naming the option with `id`; `null` when none has it. */
function named(kind: ConditionKind, id: string, options: ConditionOptions): Condition | null {
  switch (kind) {
    case 'template_is': {
      const templateId = find(options.templates, id);
      return templateId === undefined ? null : { kind, templateId };
    }
    case 'has_tag':
    case 'lacks_tag': {
      const tagId = find(options.tags, id);
      return tagId === undefined ? null : { kind, tagId };
    }
    case 'in_folder': {
      const folderId = find(options.folders, id);
      return folderId === undefined ? null : { kind, folderId };
    }
    case 'has_participant':
    case 'lacks_participant': {
      const personId = find(options.people, id);
      return personId === undefined ? null : { kind, personId };
    }
    case 'calendar_is': {
      const calendarId = find(options.calendars, id);
      return calendarId === undefined ? null : { kind, calendarId };
    }
    default:
      return null;
  }
}

/** A new condition of `kind`, pointing at the first option when it needs one. */
export function startCondition(kind: ConditionKind, options: ConditionOptions): Condition | null {
  const first = listOf(kind, options)?.[0];
  return blankCondition(kind) ?? (first === undefined ? null : named(kind, first.id, options));
}

/** `condition` pointed at option `id`; unchanged when it does not point. */
export function pointAt(condition: Condition, id: string, options: ConditionOptions): Condition {
  return named(condition.kind, id, options) ?? condition;
}

/** The options a pointing condition picks from, and the one it names. */
export function pointing(
  condition: Condition,
  options: ConditionOptions,
): { choices: ConditionOption[]; value: string } | null {
  const choices = listOf(condition.kind, options);
  const value = pointedId(condition);
  return choices === null || value === null ? null : { choices, value };
}
