/**
 * What steps can point at (insights, tags, folders), read through their
 * features, with labels for the pickers.
 */
import { useFolderTree } from '@/features/folders';
import { useInsights } from '@/features/insights';
import { useTags } from '@/features/tags';
import type { Option } from '@/ui/option-select';

import type { StepChoices } from '../draft';
import type { FolderId, InsightId, TagId } from '../types';

/** The pickers' options, by what they name; empty while they load. */
export interface StepOptions {
  insights: Option<InsightId>[];
  tags: Option<TagId>[];
  folders: Option<FolderId>[];
}

/** Every insight, tag and folder, as the step pickers list them. */
export function useStepOptions(): StepOptions {
  const insights = useInsights().data ?? [];
  const tags = useTags().data ?? [];
  const folders = useFolderTree().data?.entries ?? [];
  return {
    insights: insights.map((summary) => ({ value: summary.def.id, label: summary.def.name })),
    tags: tags.map((tag) => ({ value: tag.id, label: tag.name })),
    folders: folders.map((entry) => ({
      value: entry.folder.id,
      label: `${' '.repeat(entry.depth)}${entry.folder.name}`,
    })),
  };
}

/** The IDs new steps can start pointing at. */
export function choicesOf(options: StepOptions): StepChoices {
  return {
    insights: options.insights.map((option) => option.value),
    tags: options.tags.map((option) => option.value),
    folders: options.folders.map((option) => option.value),
  };
}
