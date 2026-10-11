/**
 * Which meetings an automatic insight runs after: groups of conditions,
 * all of a group (and), any group (or), as AUT-01's automations will use.
 */
import { Button } from '@/ui/button';
import { ICON_PROPS, Plus, X } from '@/ui/icons';

import { editorCopy } from '../copy';
import type { Condition } from '../types';
import { type ConditionOptions, startCondition, useConditionOptions } from './condition-options';
import { ConditionValue } from './condition-value';
import { type Option, OptionSelect } from './option-select';

/** Props of {@link ConditionGroups}. */
export interface ConditionGroupsProps {
  groups: Condition[][];
  onChange: (groups: Condition[][]) => void;
}

/** The kinds, in the order the menu lists them. */
const KINDS = [
  'title_contains',
  'external',
  'attendee_domain',
  'has_tag',
  'in_folder',
  'template_is',
] as const satisfies readonly Condition['kind'][];

/** `list` with item `index` replaced by `next`, or removed when `next` is `null`. */
function replaced<T>(list: readonly T[], index: number, next: T | null): T[] {
  return next === null
    ? list.filter((_, at) => at !== index)
    : list.map((item, at) => (at === index ? next : item));
}

/** The groups, each with its conditions, then "Or another group". */
export function ConditionGroups({ groups, onChange }: ConditionGroupsProps) {
  const options = useConditionOptions();
  const shown = groups.length === 0 ? [[]] : groups;
  const fresh = startCondition('title_contains', options);
  return (
    <div className="grid gap-2">
      <p className="text-xs text-fg-muted">{editorCopy.conditionsHint}</p>
      {shown.map((group, index) => (
        <div key={index} className="grid gap-2">
          {index > 0 ? (
            <p className="text-xs font-semibold text-fg-subtle">{editorCopy.or}</p>
          ) : null}
          <Group
            group={group}
            options={options}
            onChange={(next) => {
              onChange(replaced(shown, index, next));
            }}
          />
        </div>
      ))}
      {fresh === null ? null : (
        <AddButton
          label={editorCopy.addGroup}
          onClick={() => {
            onChange([...shown, [fresh]]);
          }}
        />
      )}
    </div>
  );
}

/** A ghost "+ …" button. */
function AddButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <div>
      <Button type="button" variant="ghost" size="sm" onClick={onClick}>
        <Plus {...ICON_PROPS} />
        {label}
      </Button>
    </div>
  );
}

/** One group's conditions and "Add condition". */
function Group({
  group,
  options,
  onChange,
}: {
  group: Condition[];
  options: ConditionOptions;
  onChange: (group: Condition[]) => void;
}) {
  const fresh = startCondition('title_contains', options);
  return (
    <div className="grid gap-2">
      {group.map((condition, index) => (
        <ConditionRow
          key={index}
          condition={condition}
          options={options}
          onChange={(next) => {
            onChange(replaced(group, index, next));
          }}
        />
      ))}
      {fresh === null ? null : (
        <AddButton
          label={editorCopy.addCondition}
          onClick={() => {
            onChange([...group, fresh]);
          }}
        />
      )}
    </div>
  );
}

/** A condition: its kind, its value, and remove (`null`). */
function ConditionRow({
  condition,
  options,
  onChange,
}: {
  condition: Condition;
  options: ConditionOptions;
  onChange: (condition: Condition | null) => void;
}) {
  const kinds: Option<Condition['kind']>[] = KINDS.map((kind) => ({
    value: kind,
    label: editorCopy.conditions[kind],
    disabled: startCondition(kind, options) === null,
  }));
  return (
    <div className="grid grid-cols-[10rem_1fr_auto] items-center gap-2">
      <OptionSelect
        label={editorCopy.conditions[condition.kind]}
        value={condition.kind}
        options={kinds}
        onChange={(kind) => {
          const next = startCondition(kind, options);
          if (next !== null) onChange(next);
        }}
      />
      <ConditionValue condition={condition} options={options} onChange={onChange} />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        aria-label={editorCopy.removeCondition}
        onClick={() => {
          onChange(null);
        }}
      >
        <X {...ICON_PROPS} />
      </Button>
    </div>
  );
}
