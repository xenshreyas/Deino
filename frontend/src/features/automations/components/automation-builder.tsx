/**
 * The builder: name, switches, condition groups and the step list, with
 * Save, Test on latest meeting, Duplicate and Delete. Remount it with a
 * `key` to start over from another automation.
 */
import { type FormEvent, type ReactNode, useState } from 'react';

import { ConditionGroups } from '@/features/conditions';
import { Button } from '@/ui/button';
import { Label, Switch } from '@/ui/controls';
import { IconButton } from '@/ui/icon-button';
import { CircleAlert, Copy, FlaskConical, ICON_PROPS, Trash2 } from '@/ui/icons';
import { Input } from '@/ui/input';

import { builderCopy } from '../copy';
import { blankDraft, type Draft, toDraft, toInput } from '../draft';
import { useShellScripts } from '../hooks/use-shell-scripts';
import {
  useDeleteAutomation,
  useDuplicateAutomation,
  useSaveAutomation,
  useTestAutomation,
} from '../queries';
import type { Automation, AutomationId } from '../types';
import { StepList } from './step-list';

/** Props of {@link AutomationBuilder}. */
export interface AutomationBuilderProps {
  /** The automation to edit; `null` for a new one. */
  automation: Automation | null;
  /** After a save, duplicate or delete: the automation to show next. */
  onSelect: (id: AutomationId | null) => void;
}

/** The form. */
export function AutomationBuilder({ automation, onSelect }: AutomationBuilderProps) {
  const { draft, patch, submit, error } = useBuilder(automation, onSelect);
  const shell = useShellScripts();
  return (
    <form className="grid gap-6" onSubmit={submit}>
      <NameRow draft={draft} automation={automation} onSelect={onSelect} onChange={patch} />
      <Switches draft={draft} onChange={patch} />
      <Section title={builderCopy.when}>
        <ConditionGroups
          groups={draft.conditions}
          onChange={(conditions) => {
            patch({ conditions });
          }}
        />
      </Section>
      <Section title={builderCopy.steps}>
        <StepList
          steps={draft.steps}
          shellAllowed={shell.allowed === true}
          onChange={(steps) => {
            patch({ steps });
          }}
        />
      </Section>
      {error === null ? null : <ErrorLine message={error.message} />}
    </form>
  );
}

/** The draft, its edits, and saving it. */
function useBuilder(automation: Automation | null, onSelect: (id: AutomationId | null) => void) {
  const [draft, setDraft] = useState<Draft>(() =>
    automation === null ? blankDraft() : toDraft(automation),
  );
  const save = useSaveAutomation();
  return {
    draft,
    error: save.error,
    patch: (change: Partial<Draft>) => {
      setDraft((current) => ({ ...current, ...change }));
    },
    submit: (event: FormEvent) => {
      event.preventDefault();
      save.mutate(
        { id: automation?.id ?? null, input: toInput(draft) },
        {
          onSuccess: (saved) => {
            onSelect(saved.id);
          },
        },
      );
    },
  };
}

/** A titled part of the form. */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold text-fg">{title}</h3>
      {children}
    </section>
  );
}

/** The name and the actions. */
function NameRow({
  draft,
  automation,
  onSelect,
  onChange,
}: {
  draft: Draft;
  automation: Automation | null;
  onSelect: (id: AutomationId | null) => void;
  onChange: (change: Partial<Draft>) => void;
}) {
  return (
    <div className="flex items-end gap-2">
      <div className="grid flex-1 gap-1.5">
        <Label htmlFor="automation-name">{builderCopy.name}</Label>
        <Input
          id="automation-name"
          placeholder={builderCopy.namePlaceholder}
          value={draft.name}
          onChange={(event) => {
            onChange({ name: event.target.value });
          }}
        />
      </div>
      <Button type="submit">{builderCopy.save}</Button>
      {automation === null ? null : <SavedActions automation={automation} onSelect={onSelect} />}
    </div>
  );
}

/** Test on latest meeting, Duplicate and Delete, for a saved automation. */
function SavedActions({
  automation,
  onSelect,
}: {
  automation: Automation;
  onSelect: (id: AutomationId | null) => void;
}) {
  const duplicate = useDuplicateAutomation();
  const remove = useDeleteAutomation();
  return (
    <>
      <TestButton id={automation.id} />
      <IconButton
        icon={Copy}
        label={builderCopy.duplicate}
        onClick={() => {
          duplicate.mutate(automation.id, {
            onSuccess: (copy) => {
              onSelect(copy.id);
            },
          });
        }}
      />
      <IconButton
        icon={Trash2}
        label={builderCopy.delete}
        onClick={() => {
          remove.mutate(automation.id, {
            onSuccess: () => {
              onSelect(null);
            },
          });
        }}
      />
    </>
  );
}

/** "Test on latest meeting": a manual run on the newest finished meeting. */
function TestButton({ id }: { id: AutomationId }) {
  const test = useTestAutomation();
  return (
    <Button
      type="button"
      variant="secondary"
      disabled={test.isPending}
      title={test.isSuccess ? builderCopy.tested : undefined}
      onClick={() => {
        test.mutate(id);
      }}
    >
      <FlaskConical {...ICON_PROPS} />
      {builderCopy.test}
    </Button>
  );
}

/** On, run automatically, stop on failure. */
function Switches({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (change: Partial<Draft>) => void;
}) {
  const rows = [
    { id: 'enabled', label: builderCopy.enabled, value: draft.enabled },
    { id: 'autoRun', label: builderCopy.autoRun, value: draft.autoRun },
    { id: 'stopOnFailure', label: builderCopy.stopOnFailure, value: draft.stopOnFailure },
  ] as const;
  return (
    <div className="grid gap-2">
      {rows.map((row) => (
        <div key={row.id} className="flex items-center gap-2">
          <Switch
            id={`automation-${row.id}`}
            checked={row.value}
            onCheckedChange={(checked) => {
              onChange({ [row.id]: checked });
            }}
          />
          <Label htmlFor={`automation-${row.id}`}>{row.label}</Label>
        </div>
      ))}
    </div>
  );
}

/** A save error, in words the user can act on. */
function ErrorLine({ message }: { message: string }) {
  return (
    <p role="alert" className="flex items-start gap-1.5 text-xs text-danger">
      <CircleAlert {...ICON_PROPS} size={14} className="mt-px shrink-0" />
      {message}
    </p>
  );
}
