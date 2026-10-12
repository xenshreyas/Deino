/**
 * The builder's draft: an {@link AutomationInput} as the form edits it
 * (steps keyed for React), new steps of each kind, and the conversions
 * both ways. Rust validates the result.
 */
import type {
  Action,
  ActionKind,
  Automation,
  AutomationInput,
  FolderId,
  InsightId,
  PayloadFields,
  TagId,
} from './types';

/** One step row of the form. */
export interface StepDraft {
  /** Stable key for React lists; not saved. */
  key: string;
  action: Action;
}

/** The whole form. */
export interface Draft extends Omit<AutomationInput, 'steps'> {
  steps: StepDraft[];
}

/** What steps that name something can start pointing at. */
export interface StepChoices {
  insights: InsightId[];
  tags: TagId[];
  folders: FolderId[];
}

/** The payload a new script step sends. */
export const DEFAULT_FIELDS: PayloadFields = {
  notes: true,
  actionItems: true,
  transcript: false,
  insights: true,
  attendees: true,
  link: true,
  audioPath: false,
};

/** A fresh key for a new row. */
export function rowKey(): string {
  return crypto.randomUUID();
}

/** A blank automation: on, automatic, every meeting, no steps. */
export function blankDraft(): Draft {
  return {
    name: '',
    enabled: true,
    autoRun: true,
    stopOnFailure: false,
    conditions: [],
    steps: [],
  };
}

/** The form for a stored automation. */
export function toDraft(automation: Automation): Draft {
  return {
    name: automation.name,
    enabled: automation.enabled,
    autoRun: automation.autoRun,
    stopOnFailure: automation.stopOnFailure,
    conditions: automation.conditions,
    steps: automation.steps.map((action) => ({ key: rowKey(), action })),
  };
}

/** The input to save; empty condition groups are left out. */
export function toInput(draft: Draft): AutomationInput {
  return {
    name: draft.name,
    enabled: draft.enabled,
    autoRun: draft.autoRun,
    stopOnFailure: draft.stopOnFailure,
    conditions: draft.conditions.filter((group) => group.length > 0),
    steps: draft.steps.map((step) => step.action),
  };
}

/**
 * A new step of `kind`, pointing at the first insight, tag or folder when
 * it needs one; `null` when there is nothing to point at, or the kind is
 * not built yet.
 */
export function startAction(kind: ActionKind, choices: StepChoices): Action | null {
  switch (kind) {
    case 'generate_insight':
      return first(choices.insights, (insightId) => ({ kind, insightId }));
    case 'add_tag':
      return first(choices.tags, (tagId) => ({ kind, tagId }));
    case 'add_to_folder':
      return first(choices.folders, (folderId) => ({ kind, folderId }));
    case 'export_file':
      return { kind, format: 'markdown', folder: '' };
    case 'notify':
      return { kind, message: '' };
    case 'run_shell':
      return { kind, command: '', fields: DEFAULT_FIELDS };
    default:
      return null;
  }
}

/** `make(ids[0])` when there is one. */
function first<T extends string>(ids: readonly T[], make: (id: T) => Action): Action | null {
  const id = ids[0];
  return id === undefined ? null : make(id);
}

/** `steps` with the step at `index` moved by `offset` (−1 up, +1 down). */
export function moved<T>(steps: readonly T[], index: number, offset: -1 | 1): T[] {
  const target = index + offset;
  const item = steps[index];
  const other = steps[target];
  if (item === undefined || other === undefined) return [...steps];
  return steps.map((step, at) => {
    if (at === index) return other;
    if (at === target) return item;
    return step;
  });
}
