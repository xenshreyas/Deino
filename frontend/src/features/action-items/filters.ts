/**
 * Filters as the filter bar shows them: the active conditions as removable
 * chips, and every condition the `F` menu offers. Pure: options and "now"
 * come in, filters go out; the list query does the filtering in Rust.
 */
import { format, parseISO, subDays } from 'date-fns';

import { copy } from './copy';
import { toDay } from './dates';
import type {
  ActionItemFilter,
  AssigneeScope,
  DateRange,
  DueFilter,
  FilterOptions,
  StatusFilter,
} from './types';

/** A condition of a filter, by field. */
export type FilterField =
  'assignee' | 'status' | 'due' | 'dateRange' | 'tagId' | 'companyId' | 'folderId' | 'meetingId';

/** One active condition, as a chip. */
export interface FilterChip {
  field: FilterField;
  label: string;
}

/** One condition the filter menu offers. */
export interface FilterChoice {
  id: string;
  group: string;
  label: string;
  apply: (filter: ActionItemFilter) => ActionItemFilter;
}

const STATUSES: readonly StatusFilter[] = ['open', 'done', 'any'];
const DUES: readonly DueFilter[] = ['overdue', 'today', 'this_week', 'no_date'];

/** A canonical key, equal for equal filters whatever their key order. */
export function filterKey(filter: ActionItemFilter): string {
  return JSON.stringify([
    filter.assignee,
    filter.status,
    filter.meetingId,
    filter.tagId,
    filter.companyId,
    filter.folderId,
    filter.due,
    filter.dateRange,
  ]);
}

/** `filter` without the condition on `field` (it then shows everything). */
export function without(filter: ActionItemFilter, field: FilterField): ActionItemFilter {
  if (field === 'assignee') return { ...filter, assignee: { kind: 'anyone' } };
  if (field === 'status') return { ...filter, status: 'any' };
  return { ...filter, [field]: null };
}

/** Every condition of `filter` that narrows the list, labelled. */
export function activeChips(filter: ActionItemFilter, options: FilterOptions): FilterChip[] {
  const chips: FilterChip[] = [];
  const add = (field: FilterField, label: string | null) => {
    if (label !== null) chips.push({ field, label });
  };
  add('assignee', filter.assignee.kind === 'anyone' ? null : scopeLabel(filter.assignee, options));
  add('status', filter.status === 'any' ? null : copy.filter.status[filter.status]);
  add('due', filter.due === null ? null : copy.filter.due[filter.due]);
  add('dateRange', filter.dateRange === null ? null : rangeLabel(filter.dateRange));
  add('tagId', named(options.tags, filter.tagId, copy.filter.tagged));
  add('companyId', named(options.companies, filter.companyId, copy.filter.with));
  add('folderId', named(options.folders, filter.folderId, copy.filter.in));
  add('meetingId', filter.meetingId === null ? null : copy.filter.meeting);
  return chips;
}

/** The label of the option `id` names, or `null` when there is no condition. */
function named(
  list: readonly { id: string; name: string }[],
  id: string | null,
  label: (name: string) => string,
): string | null {
  if (id === null) return null;
  return label(list.find((option) => option.id === id)?.name ?? '…');
}

function scopeLabel(scope: AssigneeScope, options: FilterOptions): string {
  if (scope.kind !== 'person') return copy.filter.scope[scope.kind];
  const person = options.people.find((candidate) => candidate.id === scope.personId);
  return copy.filter.scope.person(person?.name ?? '…');
}

function rangeLabel(range: DateRange): string {
  const show = (day: string) => format(parseISO(day), 'MMM d');
  if (range.from !== null && range.to !== null) {
    return copy.filter.held.between(show(range.from), show(range.to));
  }
  if (range.from !== null) return copy.filter.held.since(show(range.from));
  return range.to === null ? '' : copy.filter.held.until(show(range.to));
}

/** A choice that sets `patch` on the filter. */
function choice(id: string, group: string, label: string, patch: Partial<ActionItemFilter>) {
  return { id, group, label, apply: (filter: ActionItemFilter) => ({ ...filter, ...patch }) };
}

/** Mine, others, unassigned, anyone, then each other person. */
function scopeChoices(options: FilterOptions): FilterChoice[] {
  const group = copy.filter.groups.assignee;
  const scopes = (['me', 'others', 'unassigned', 'anyone'] as const).map((kind) =>
    choice(`assignee:${kind}`, group, copy.filter.scope[kind], { assignee: { kind } }),
  );
  const people = options.people
    .filter((person) => !person.isMe)
    .map((person) =>
      choice(`assignee:${person.id}`, group, person.name, {
        assignee: { kind: 'person', personId: person.id },
      }),
    );
  return [...scopes, ...people];
}

/** Statuses, due windows and meeting-date presets. */
function stateChoices(now: Date): FilterChoice[] {
  const groups = copy.filter.groups;
  const since = (days: number): DateRange => ({ from: toDay(subDays(now, days - 1)), to: null });
  return [
    ...STATUSES.map((status) =>
      choice(`status:${status}`, groups.status, copy.filter.status[status], { status }),
    ),
    ...DUES.map((due) => choice(`due:${due}`, groups.due, copy.filter.due[due], { due })),
    choice('held:7', groups.held, copy.filter.held.week, { dateRange: since(7) }),
    choice('held:30', groups.held, copy.filter.held.month, { dateRange: since(30) }),
  ];
}

/** Tags, companies and folders. */
function entityChoices(options: FilterOptions): FilterChoice[] {
  const groups = copy.filter.groups;
  return [
    ...options.tags.map((tag) => choice(`tag:${tag.id}`, groups.tag, tag.name, { tagId: tag.id })),
    ...options.companies.map((company) =>
      choice(`company:${company.id}`, groups.company, company.name, { companyId: company.id }),
    ),
    ...options.folders.map((folder) =>
      choice(`folder:${folder.id}`, groups.folder, folder.name, { folderId: folder.id }),
    ),
  ];
}

/** Every condition the filter menu offers, grouped, seen from `now`. */
export function filterChoices(options: FilterOptions, now: Date): FilterChoice[] {
  return [...scopeChoices(options), ...stateChoices(now), ...entityChoices(options)];
}
