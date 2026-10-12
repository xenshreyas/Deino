/**
 * "Add step": the ready steps, each with what it does, then the planned
 * provider steps marked "WIP" (PARITY_PLAN D12). A ready step that cannot
 * start yet (nothing to name, or scripts are off) stays listed, disabled,
 * with the reason in place of its description.
 *
 * Each item's accessible name is the step's name; its description (or the
 * reason) is its accessible description.
 */
import { useId } from 'react';

import { Button } from '@/ui/button';
import { ICON_PROPS, Plus } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { builderCopy } from '../copy';
import { startAction } from '../draft';
import type { Action, StepInfo } from '../types';
import { blockedBecause, choicesOf, type StepOptions } from './step-choices';
import { stepIcon } from './step-icons';

/** Props of {@link StepPicker}. */
export interface StepPickerProps {
  catalog: readonly StepInfo[];
  options: StepOptions;
  /** Whether "Allow shell scripts" is on; a script step is blocked until it is. */
  shellAllowed: boolean;
  onPick: (action: Action) => void;
}

/** The "Add step" button and its menu. */
export function StepPicker({ catalog, options, shellAllowed, onPick }: StepPickerProps) {
  const choices = choicesOf(options);
  const planned = catalog.filter((entry) => entry.availability === 'wip');
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="secondary" size="sm" className="justify-self-start">
          <Plus {...ICON_PROPS} />
          {builderCopy.addStep}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-80">
        {catalog
          .filter((entry) => entry.availability === 'ready')
          .map((entry) => {
            const action = startAction(entry.kind, choices);
            const reason = blockedBecause(entry.kind, action, shellAllowed);
            return (
              <ReadyItem
                key={entry.kind}
                entry={entry}
                reason={reason}
                onSelect={() => {
                  if (action !== null && reason === null) onPick(action);
                }}
              />
            );
          })}
        {planned.length === 0 ? null : <DropdownMenuSeparator />}
        {planned.length === 0 ? null : (
          <DropdownMenuLabel>{builderCopy.comingLater}</DropdownMenuLabel>
        )}
        {planned.map((entry) => (
          <PlannedItem key={entry.kind} entry={entry} />
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** A ready step: icon, name, and its description or why it is blocked. */
function ReadyItem({
  entry,
  reason,
  onSelect,
}: {
  entry: StepInfo;
  reason: string | null;
  onSelect: () => void;
}) {
  const describedBy = useId();
  const StepIcon = stepIcon(entry.kind);
  return (
    <DropdownMenuItem
      aria-label={entry.label}
      aria-describedby={describedBy}
      disabled={reason !== null}
      onSelect={onSelect}
      className="group h-auto items-start py-1.5 data-disabled:opacity-100"
    >
      <StepIcon {...ICON_PROPS} className="mt-0.5 shrink-0 group-data-disabled:opacity-50" />
      <span className="grid min-w-0 flex-1 gap-0.5">
        <span className="group-data-disabled:text-fg-subtle">{entry.label}</span>
        <span id={describedBy} className="text-xs text-fg-muted">
          {reason ?? entry.description}
        </span>
      </span>
    </DropdownMenuItem>
  );
}

/** A planned provider step: listed, disabled, with a quiet "WIP" badge. */
function PlannedItem({ entry }: { entry: StepInfo }) {
  const StepIcon = stepIcon(entry.kind);
  return (
    <DropdownMenuItem aria-label={entry.label} disabled>
      <StepIcon {...ICON_PROPS} />
      <span className="flex-1 truncate">{entry.label}</span>
      <span className="inline-flex h-5 items-center rounded-sm bg-bg-muted px-1.5 text-xs font-semibold text-fg-muted">
        {builderCopy.wip}
      </span>
    </DropdownMenuItem>
  );
}
