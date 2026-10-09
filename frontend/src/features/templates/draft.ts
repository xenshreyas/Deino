/**
 * The template editor's working copy: {@link TemplateContent} with stable
 * keys for every section and rule (so reordering keeps focus and state),
 * and rule fields kept as typed text until save. Pure functions only; Rust
 * validates and normalises on save.
 */
import type { Audience, AutoRule, SectionFormat, TemplateContent, TemplateSection } from './types';

/** A section being edited. */
export interface DraftSection extends TemplateSection {
  key: string;
}

/** An auto rule being edited, its numbers and domains as typed. */
export interface DraftRule {
  key: string;
  label: string;
  titlePattern: string;
  audience: Audience | null;
  domains: string;
  min: string;
  max: string;
}

/** A template being edited. */
export interface Draft {
  name: string;
  description: string;
  meetingContext: string;
  sections: DraftSection[];
  rules: DraftRule[];
}

let lastKey = 0;

/** A key unique within this page's lifetime. */
function nextKey(): string {
  lastKey += 1;
  return `draft-${String(lastKey)}`;
}

/** A section with `title` and nothing else. */
export function newSection(title: string, format: SectionFormat = 'bullets'): DraftSection {
  return { key: nextKey(), title, instructions: '', format };
}

/** An empty rule. */
export function newRule(): DraftRule {
  return {
    key: nextKey(),
    label: '',
    titlePattern: '',
    audience: null,
    domains: '',
    min: '',
    max: '',
  };
}

/** A blank template with one section. */
export function emptyDraft(sectionTitle: string): Draft {
  return {
    name: '',
    description: '',
    meetingContext: '',
    sections: [newSection(sectionTitle)],
    rules: [],
  };
}

/** The editable form of saved content. */
export function toDraft(content: TemplateContent): Draft {
  return {
    name: content.name,
    description: content.description,
    meetingContext: content.meetingContext,
    sections: content.sections.map((section) => ({ ...section, key: nextKey() })),
    rules: content.autoRules.map(ruleToDraft),
  };
}

function ruleToDraft(rule: AutoRule): DraftRule {
  return {
    key: nextKey(),
    label: rule.label,
    titlePattern: rule.titlePattern ?? '',
    audience: rule.audience,
    domains: rule.attendeeDomains.join(', '),
    min: rule.attendeeCount?.min?.toString() ?? '',
    max: rule.attendeeCount?.max?.toString() ?? '',
  };
}

/** The content to save. */
export function fromDraft(draft: Draft): TemplateContent {
  return {
    name: draft.name,
    description: draft.description,
    meetingContext: draft.meetingContext,
    sections: draft.sections.map(({ title, instructions, format }) => ({
      title,
      instructions,
      format,
    })),
    autoRules: draft.rules.map(ruleFromDraft),
  };
}

function ruleFromDraft(rule: DraftRule): AutoRule {
  const min = count(rule.min);
  const max = count(rule.max);
  return {
    label: rule.label,
    titlePattern: rule.titlePattern.trim() === '' ? null : rule.titlePattern,
    attendeeDomains: rule.domains.split(/[\s,]+/).filter((domain) => domain !== ''),
    audience: rule.audience,
    attendeeCount: min === null && max === null ? null : { min, max },
  };
}

/** A whole number typed into a count field, or `null` when blank or not a number. */
function count(text: string): number | null {
  const value = Number.parseInt(text, 10);
  return Number.isInteger(value) ? value : null;
}

/** `items` with the item at `from` moved to `to` (clamped to the list). */
export function move<T>(items: readonly T[], from: number, to: number): T[] {
  const target = Math.max(0, Math.min(items.length - 1, to));
  const moved = items[from];
  if (moved === undefined || target === from) return [...items];
  const rest = items.filter((_, index) => index !== from);
  return [...rest.slice(0, target), moved, ...rest.slice(target)];
}

/** `items` with the one keyed `key` changed by `patch`. */
export function patchItem<T extends { key: string }>(
  items: readonly T[],
  key: string,
  patch: Partial<T>,
): T[] {
  return items.map((item) => (item.key === key ? { ...item, ...patch } : item));
}

/** Section titles as heading lines, the scaffold the `/template` command inserts. */
export function scaffoldHeadings(content: TemplateContent): string[] {
  return content.sections.map((section) => section.title);
}
