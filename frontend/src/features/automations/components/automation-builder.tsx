/**
 * The builder: the name with Save (and, once saved, Test on latest meeting
 * and ⋯ → Duplicate, Delete), the switches, condition groups, the step
 * list, and for a saved automation its run log. Remount it with a `key` to
 * start over from another automation.
 */
import { type FormEvent, type ReactNode, useState } from 'react';

import { ConditionGroups } from '@/features/conditions';
import { Button } from '@/ui/button';
import { Label, Switch } from '@/ui/controls';
import { Check, CircleAlert, ICON_PROPS } from '@/ui/icons';
import { Input } from '@/ui/input';

import { builderCopy, runCopy } from '../copy';
import { blankDraft, type Draft, toDraft, toInput } from '../draft';
import { useShellScripts } from '../hooks/use-shell-scripts';
import { useSaveAutomation } from '../queries';
import type { Automation, AutomationId } from '../types';
import { SavedActions } from './builder-actions';
import { RecentRuns } from './recent-runs';
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
  const builder = useBuilder(automation, onSelect);
  const { draft, patch } = builder;
  const shell = useShellScripts();
  return (
    <form className="grid gap-8" onSubmit={builder.submit}>
      <div className="grid gap-4">
        <TitleRow builder={builder} automation={automation} onSelect={onSelect} />
        <Switches draft={draft} onChange={patch} />
        {builder.error === null ? null : <ErrorLine message={builder.error.message} />}
      </div>
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
      {automation === null ? null : (
        <Section title={runCopy.recent}>
          <RecentRuns automationId={automation.id} />
        </Section>
      )}
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
    saving: save.isPending,
    saved: save.isSuccess,
    patch: (change: Partial<Draft>) => {
      save.reset();
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

type Builder = ReturnType<typeof useBuilder>;

/** A titled part of the form. */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-3">
      <h3 className="text-sm font-semibold text-fg">{title}</h3>
      {children}
    </section>
  );
}

/** The name, edited in place like a title, and the actions. */
function TitleRow({
  builder,
  automation,
  onSelect,
}: {
  builder: Builder;
  automation: Automation | null;
  onSelect: (id: AutomationId | null) => void;
}) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
      <Input
        aria-label={builderCopy.name}
        placeholder={builderCopy.namePlaceholder}
        value={builder.draft.name}
        onChange={(event) => {
          builder.patch({ name: event.target.value });
        }}
        className="-ml-2.5 h-10 min-w-[14rem] flex-1 border-transparent bg-transparent text-base font-semibold"
      />
      <div className="ml-auto flex items-center gap-2">
        {automation === null ? null : <SavedActions automation={automation} onSelect={onSelect} />}
        <Button type="submit" disabled={builder.saving}>
          {builder.saved ? <Check {...ICON_PROPS} /> : null}
          {builder.saved ? builderCopy.saved : builderCopy.save}
        </Button>
      </div>
    </div>
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
    <div className="grid gap-2.5">
      {rows.map((row) => (
        <div key={row.id} className="flex items-center gap-2.5">
          <Switch
            id={`automation-${row.id}`}
            checked={row.value}
            onCheckedChange={(checked) => {
              onChange({ [row.id]: checked });
            }}
          />
          <Label htmlFor={`automation-${row.id}`} className="font-normal text-fg">
            {row.label}
          </Label>
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
