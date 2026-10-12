/**
 * The Meetings filter as the page edits it: a stable key, the active
 * conditions as chips, and every condition the `F` menu offers. Pure, over
 * the typed `MeetingFilter` Rust takes.
 */
import { format, subDays } from 'date-fns';

import { copy } from './copy';
import { ALL_MEETINGS, type FilterOptions, type MeetingFilter } from './types';

/** A condition the filter can carry. */
export type FilterField = Exclude<keyof MeetingFilter, never>;

/** One active condition, as a removable chip. */
export interface FilterChip {
  field: FilterField;
  label: string;
}

/** One entry of the filter menu. */
export interface FilterChoice {
  id: string;
  group: string;
  label: string;
  apply: (filter: MeetingFilter) => MeetingFilter;
}

/** A key equal for equal filters, whatever the order of their lists. */
export function filterKey(filter: MeetingFilter): string {
  const sorted = (values: readonly string[]) => [...values].sort();
  return JSON.stringify({
    ...filter,
    people: sorted(filter.people),
    companies: sorted(filter.companies),
    tags: sorted(filter.tags),
    folders: sorted(filter.folders),
  });
}

/** True when nothing narrows the list. */
export function isAllMeetings(filter: MeetingFilter): boolean {
  return filterKey(filter) === filterKey(ALL_MEETINGS);
}

/** `filter` without the condition `field`. */
export function without(filter: MeetingFilter, field: FilterField): MeetingFilter {
  return { ...filter, [field]: ALL_MEETINGS[field] };
}

/** `values` with `value` added, or removed when it is there. */
function toggled<T extends string>(values: readonly T[], value: T): T[] {
  return values.includes(value) ? values.filter((v) => v !== value) : [...values, value];
}

/** The names of `ids` among `options`, joined. */
function names(ids: readonly string[], options: readonly { id: string; name: string }[]): string {
  return ids.map((id) => options.find((option) => option.id === id)?.name ?? '…').join(', ');
}

/** The active conditions, in the menu's order. */
export function activeChips(filter: MeetingFilter, options: FilterOptions): FilterChip[] {
  const chips: FilterChip[] = [];
  const range = filter.dateRange;
  if (range !== null)
    chips.push({ field: 'dateRange', label: copy.filter.range(range.from, range.to) });
  if (filter.people.length > 0)
    chips.push({ field: 'people', label: names(filter.people, options.people) });
  if (filter.companies.length > 0)
    chips.push({ field: 'companies', label: names(filter.companies, options.companies) });
  if (filter.tags.length > 0)
    chips.push({ field: 'tags', label: names(filter.tags, options.tags) });
  if (filter.folders.length > 0)
    chips.push({ field: 'folders', label: names(filter.folders, options.folders) });
  if (filter.hasActionItems) chips.push({ field: 'hasActionItems', label: copy.filter.hasItems });
  if (filter.audience !== null)
    chips.push({ field: 'audience', label: copy.filter.audience[filter.audience] });
  return chips;
}

/** "Today", "Last 7 days" and "Last 30 days", as of `now`. */
function dateChoices(now: Date): FilterChoice[] {
  const day = (date: Date) => format(date, 'yyyy-MM-dd');
  return copy.filter.presets.map(({ label, days }) => ({
    id: `date:${days.toString()}`,
    group: copy.filter.groups.date,
    label,
    apply: (filter) => ({ ...filter, dateRange: { from: day(subDays(now, days)), to: day(now) } }),
  }));
}

/** One toggle per option of a list field. */
function listChoices(
  field: 'people' | 'companies' | 'tags' | 'folders',
  options: readonly { id: string; name: string }[],
): FilterChoice[] {
  return options.map((option) => ({
    id: `${field}:${option.id}`,
    group: copy.filter.groups[field],
    label: option.name,
    apply: (filter) => ({ ...filter, [field]: toggled<string>(filter[field], option.id) }),
  }));
}

/** Action items and internal or external. */
function flagChoices(): FilterChoice[] {
  const group = copy.filter.groups.more;
  return [
    {
      id: 'items',
      group,
      label: copy.filter.hasItems,
      apply: (f) => ({ ...f, hasActionItems: !f.hasActionItems }),
    },
    {
      id: 'internal',
      group,
      label: copy.filter.audience.internal,
      apply: (f) => ({ ...f, audience: 'internal' }),
    },
    {
      id: 'external',
      group,
      label: copy.filter.audience.external,
      apply: (f) => ({ ...f, audience: 'external' }),
    },
  ];
}

/** Every condition the menu offers, grouped, as of `now`. */
export function filterChoices(options: FilterOptions, now: Date): FilterChoice[] {
  return [
    ...dateChoices(now),
    ...listChoices('people', options.people),
    ...listChoices('companies', options.companies),
    ...listChoices('tags', options.tags),
    ...listChoices('folders', options.folders),
    ...flagChoices(),
  ];
}
