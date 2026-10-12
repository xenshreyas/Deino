/**
 * What steps can point at (insights, tags, folders, webhooks), read through
 * their features, with labels for the pickers.
 */
import { useFolderTree } from '@/features/folders';
import { useInsights } from '@/features/insights';
import { useTags } from '@/features/tags';
import { useWebhooks } from '@/features/webhooks';
import type { Option } from '@/ui/option-select';

import { builderCopy } from '../copy';
import type { StepChoices } from '../draft';
import type { Action, ActionKind, FolderId, InsightId, TagId, WebhookId } from '../types';

/** The pickers' options, by what they name; empty while they load. */
export interface StepOptions {
  insights: Option<InsightId>[];
  tags: Option<TagId>[];
  folders: Option<FolderId>[];
  webhooks: Option<WebhookId>[];
}

/** Every insight, tag, folder and webhook, as the step pickers list them. */
export function useStepOptions(): StepOptions {
  const webhooks = useWebhooks().data ?? [];
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
    webhooks: webhooks.map((webhook) => ({
      value: webhook.id,
      label: webhook.label || webhook.url,
    })),
  };
}

/**
 * Why a ready step cannot be added now, or `null` when it can: a script
 * while shell scripts are off, or a step with nothing to name yet
 * (`action` is `null` when `startAction` found no insight, tag or folder).
 */
export function blockedBecause(
  kind: ActionKind,
  action: Action | null,
  shellAllowed: boolean,
): string | null {
  if (kind === 'run_shell' && !shellAllowed) return builderCopy.needs.run_shell;
  if (action !== null) return null;
  const needs: Partial<Record<ActionKind, string>> = builderCopy.needs;
  return needs[kind] ?? null;
}

/** The IDs new steps can start pointing at. */
export function choicesOf(options: StepOptions): StepChoices {
  return {
    insights: options.insights.map((option) => option.value),
    tags: options.tags.map((option) => option.value),
    folders: options.folders.map((option) => option.value),
    webhooks: options.webhooks.map((option) => option.value),
  };
}
