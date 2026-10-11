/**
 * What a condition can name: the user's templates, tags and folders, read
 * through their features, and how a condition of each kind starts.
 */
import { type FolderId, useFolderTree } from '@/features/folders';
import { type TagId, useTags } from '@/features/tags';
import { type TemplateId, useTemplates } from '@/features/templates';

import type { Condition } from '../types';
import { blankCondition } from './draft';

/** One thing a condition can point at. */
export interface ConditionOption<Id extends string> {
  id: Id;
  label: string;
}

/** Templates, tags and folders, as the condition pickers list them. */
export interface ConditionOptions {
  templates: ConditionOption<TemplateId>[];
  tags: ConditionOption<TagId>[];
  folders: ConditionOption<FolderId>[];
}

/** The pickers' options; empty while they load. */
export function useConditionOptions(): ConditionOptions {
  const templates = useTemplates().data ?? [];
  const tags = useTags().data ?? [];
  const folders = useFolderTree().data?.entries ?? [];
  return {
    templates: templates.map((template) => ({ id: template.id, label: template.content.name })),
    tags: tags.map((tag) => ({ id: tag.id, label: tag.name })),
    folders: folders.map((entry) => ({
      id: entry.folder.id,
      label: `${' '.repeat(entry.depth)}${entry.folder.name}`,
    })),
  };
}

/** A new condition of `kind`, pointing at the first option when it needs one. */
export function startCondition(
  kind: Condition['kind'],
  options: ConditionOptions,
): Condition | null {
  const blank = blankCondition(kind);
  if (blank !== null) return blank;
  if (kind === 'template_is') {
    const first = options.templates[0];
    return first === undefined ? null : { kind, templateId: first.id };
  }
  if (kind === 'has_tag') {
    const first = options.tags[0];
    return first === undefined ? null : { kind, tagId: first.id };
  }
  const first = options.folders[0];
  return kind === 'in_folder' && first !== undefined ? { kind, folderId: first.id } : null;
}

/** `condition` pointed at the option with `id` of its own kind; unchanged otherwise. */
export function pointAt(condition: Condition, id: string, options: ConditionOptions): Condition {
  switch (condition.kind) {
    case 'template_is': {
      const found = options.templates.find((option) => option.id === id);
      return found === undefined ? condition : { ...condition, templateId: found.id };
    }
    case 'has_tag': {
      const found = options.tags.find((option) => option.id === id);
      return found === undefined ? condition : { ...condition, tagId: found.id };
    }
    case 'in_folder': {
      const found = options.folders.find((option) => option.id === id);
      return found === undefined ? condition : { ...condition, folderId: found.id };
    }
    default:
      return condition;
  }
}

/** The options a pointing condition picks from, and the one it names. */
export function pointing(
  condition: Condition,
  options: ConditionOptions,
): { choices: ConditionOption<string>[]; value: string } | null {
  switch (condition.kind) {
    case 'template_is':
      return { choices: options.templates, value: condition.templateId };
    case 'has_tag':
      return { choices: options.tags, value: condition.tagId };
    case 'in_folder':
      return { choices: options.folders, value: condition.folderId };
    default:
      return null;
  }
}
