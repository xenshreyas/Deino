/**
 * The builder's steps, in order: each with its settings, move up and down
 * and remove; then the step picker, where provider steps show as "WIP"
 * and cannot be added (PARITY_PLAN D12).
 */
import { Button } from '@/ui/button';
import { IconButton } from '@/ui/icon-button';
import { ArrowDown, ArrowUp, ICON_PROPS, Plus, X } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { builderCopy } from '../copy';
import { moved, rowKey, startAction, type StepDraft } from '../draft';
import { useStepCatalog } from '../queries';
import type { ActionKind, StepInfo } from '../types';
import { choicesOf, type StepOptions, useStepOptions } from './step-choices';
import { StepFields } from './step-fields';

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
  const label = (kind: ActionKind) => catalog.find((entry) => entry.kind === kind)?.label ?? kind;
  return (
    <div className="grid gap-2">
      {steps.length === 0 ? <p className="text-sm text-fg-muted">{builderCopy.noSteps}</p> : null}
      <ol className="grid gap-2">
        {steps.map((step, index) => (
          <li key={step.key} className="grid gap-2 rounded-md border border-border p-3">
            <StepHeader
              title={`${String(index + 1)}. ${label(step.action.kind)}`}
              index={index}
              steps={steps}
              onChange={onChange}
            />
            <StepFields
              action={step.action}
              options={options}
              shellAllowed={shellAllowed}
              onChange={(action) => {
                onChange(
                  steps.map((other) => (other.key === step.key ? { ...other, action } : other)),
                );
              }}
            />
          </li>
        ))}
      </ol>
      <StepPicker
        catalog={catalog}
        options={options}
        onPick={(action) => {
          onChange([...steps, { key: rowKey(), action }]);
        }}
      />
    </div>
  );
}

/** A step's title with move up, move down and remove. */
function StepHeader({
  title,
  index,
  steps,
  onChange,
}: {
  title: string;
  index: number;
  steps: StepDraft[];
  onChange: (steps: StepDraft[]) => void;
}) {
  return (
    <div className="flex items-center gap-1">
      <span className="flex-1 text-sm font-semibold text-fg">{title}</span>
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
    </div>
  );
}

/** "Add step": every kind, provider kinds disabled with a WIP badge. */
function StepPicker({
  catalog,
  options,
  onPick,
}: {
  catalog: StepInfo[];
  options: StepOptions;
  onPick: (action: NonNullable<ReturnType<typeof startAction>>) => void;
}) {
  const choices = choicesOf(options);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="ghost" size="sm" className="justify-self-start">
          <Plus {...ICON_PROPS} />
          {builderCopy.addStep}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-80">
        {catalog.map((entry) => {
          const action = entry.availability === 'ready' ? startAction(entry.kind, choices) : null;
          return (
            <DropdownMenuItem
              key={entry.kind}
              disabled={action === null}
              title={entry.description}
              onSelect={() => {
                if (action !== null) onPick(action);
              }}
            >
              <span className="flex-1">{entry.label}</span>
              {entry.availability === 'wip' ? <WipBadge /> : null}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The quiet "WIP" badge of a planned step. */
function WipBadge() {
  return (
    <span className="inline-flex h-5 items-center rounded-sm bg-bg-muted px-1.5 text-xs font-semibold text-fg-muted">
      {builderCopy.wip}
    </span>
  );
}
