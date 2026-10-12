/**
 * The builder's steps, in order: each with its settings, move up and down
 * and remove; then "Add step" (`step-picker.tsx`).
 */
import { IconButton } from '@/ui/icon-button';
import { ArrowDown, ArrowUp, ICON_PROPS, X } from '@/ui/icons';

import { builderCopy } from '../copy';
import { moved, rowKey, type StepDraft } from '../draft';
import { useStepCatalog } from '../queries';
import type { Action, StepInfo } from '../types';
import { useStepOptions } from './step-choices';
import { StepFields } from './step-fields';
import { stepIcon } from './step-icons';
import { StepPicker } from './step-picker';

/** Props of {@link StepList}. */
export interface StepListProps {
  steps: StepDraft[];
  shellAllowed: boolean;
  onChange: (steps: StepDraft[]) => void;
}

/** The steps and "Add step". */
export function StepList({ steps, shellAllowed, onChange }: StepListProps) {
  const options = useStepOptions();
  const catalog = useStepCatalog().data ?? [];
  return (
    <div className="grid gap-3">
      {steps.length === 0 ? (
        <p className="text-sm text-fg-muted">{builderCopy.noSteps}</p>
      ) : (
        <ol className="grid gap-2">
          {steps.map((step, index) => (
            <li key={step.key} className="grid gap-3 rounded-lg border border-border p-3">
              <StepHeader catalog={catalog} index={index} steps={steps} onChange={onChange} />
              <StepFields
                action={step.action}
                options={options}
                shellAllowed={shellAllowed}
                onChange={(action) => {
                  onChange(replaced(steps, step.key, action));
                }}
              />
            </li>
          ))}
        </ol>
      )}
      <StepPicker
        catalog={catalog}
        options={options}
        shellAllowed={shellAllowed}
        onPick={(action) => {
          onChange([...steps, { key: rowKey(), action }]);
        }}
      />
    </div>
  );
}

/** `steps` with the action of the step keyed `key` replaced. */
function replaced(steps: StepDraft[], key: string, action: Action): StepDraft[] {
  return steps.map((other) => (other.key === key ? { ...other, action } : other));
}

/** A step's number, icon and name, with move up, move down and remove. */
function StepHeader({
  catalog,
  index,
  steps,
  onChange,
}: {
  catalog: readonly StepInfo[];
  index: number;
  steps: StepDraft[];
  onChange: (steps: StepDraft[]) => void;
}) {
  const kind = steps[index]?.action.kind ?? 'notify';
  const label = catalog.find((entry) => entry.kind === kind)?.label ?? kind;
  const StepIcon = stepIcon(kind);
  return (
    <div className="-my-1 flex items-center gap-2">
      <span className="w-4 text-sm text-fg-subtle tabular-nums">{String(index + 1)}</span>
      <StepIcon {...ICON_PROPS} className="shrink-0 text-fg-muted" />
      <h4 className="flex-1 truncate text-sm font-semibold text-fg">{label}</h4>
      <StepMoves index={index} steps={steps} onChange={onChange} />
    </div>
  );
}

/** Move up, move down and remove. */
function StepMoves({
  index,
  steps,
  onChange,
}: {
  index: number;
  steps: StepDraft[];
  onChange: (steps: StepDraft[]) => void;
}) {
  return (
    <>
      <IconButton
        icon={ArrowUp}
        label={builderCopy.moveUp}
        disabled={index === 0}
        onClick={() => {
          onChange(moved(steps, index, -1));
        }}
      />
      <IconButton
        icon={ArrowDown}
        label={builderCopy.moveDown}
        disabled={index === steps.length - 1}
        onClick={() => {
          onChange(moved(steps, index, 1));
        }}
      />
      <IconButton
        icon={X}
        label={builderCopy.removeStep}
        onClick={() => {
          onChange(steps.filter((_, at) => at !== index));
        }}
      />
    </>
  );
}
