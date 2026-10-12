/**
 * The insight editor's draft: an {@link InsightInput} as the form edits it
 * (enum options as one comma-separated line, the answer count as a mode and
 * a number), and the conversions both ways. Rust validates the result.
 */
import type { Cardinality, Condition, FieldKind, InsightField, InsightInput } from '../types';

/** One field row of the form. */
export interface FieldDraft {
  /** Stable key for React lists; not saved. */
  key: string;
  name: string;
  type: FieldKind['type'];
  /** Enum options, comma-separated. */
  options: string;
}

/** When the insight runs, as the form offers it. */
export type WhenMode = 'always' | 'matching' | 'manual';

/** The whole form. */
export interface Draft {
  name: string;
  description: string;
  fields: FieldDraft[];
  countMode: Cardinality['kind'];
  count: number;
  when: WhenMode;
  /** Condition groups (used when `when` is `matching`). */
  groups: Condition[][];
}

/** A fresh key for a new row. */
export function rowKey(): string {
  return crypto.randomUUID();
}

/** An empty field row. */
export function emptyField(): FieldDraft {
  return { key: rowKey(), name: '', type: 'text', options: '' };
}

/** A blank insight. */
export const BLANK_INPUT: InsightInput = {
  name: '',
  description: '',
  fields: [],
  cardinality: { kind: 'auto' },
  applyWhen: { mode: 'automatic', conditions: [] },
};

function fieldDraft(field: InsightField): FieldDraft {
  const options = field.kind.type === 'enum' ? field.kind.options.join(', ') : '';
  return { key: rowKey(), name: field.name, type: field.kind.type, options };
}

/** The form for `input`. */
export function toDraft(input: InsightInput): Draft {
  const apply = input.applyWhen;
  const groups = apply.mode === 'automatic' ? apply.conditions : [];
  let when: WhenMode = 'manual';
  if (apply.mode === 'automatic') when = groups.length === 0 ? 'always' : 'matching';
  return {
    name: input.name,
    description: input.description,
    fields: input.fields.map(fieldDraft),
    countMode: input.cardinality.kind,
    count: input.cardinality.kind === 'fixed' ? input.cardinality.count : 1,
    when,
    groups,
  };
}

/** `"Red, Yellow ,, Green"` as `["Red", "Yellow", "Green"]`. */
export function splitOptions(line: string): string[] {
  return line
    .split(',')
    .map((option) => option.trim())
    .filter((option) => option !== '');
}

function fieldOf(field: FieldDraft): InsightField {
  const kind: FieldKind =
    field.type === 'enum'
      ? { type: 'enum', options: splitOptions(field.options) }
      : { type: field.type };
  return { name: field.name, kind };
}

/** The input to save; empty field rows are left out. */
export function toInput(draft: Draft): InsightInput {
  const groups = draft.groups.filter((group) => group.length > 0);
  return {
    name: draft.name,
    description: draft.description,
    fields: draft.fields.filter((field) => field.name.trim() !== '').map(fieldOf),
    cardinality:
      draft.countMode === 'auto' ? { kind: 'auto' } : { kind: 'fixed', count: draft.count },
    applyWhen:
      draft.when === 'manual'
        ? { mode: 'manual' }
        : { mode: 'automatic', conditions: draft.when === 'always' ? [] : groups },
  };
}

/** The condition kinds the insight editor offers (AI-06's original six). */
export const INSIGHT_CONDITION_KINDS = [
  'title_contains',
  'external',
  'attendee_domain',
  'has_tag',
  'in_folder',
  'template_is',
] as const satisfies readonly Condition['kind'][];
