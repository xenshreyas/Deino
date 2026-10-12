/**
 * Which meetings something applies to: groups of conditions, all of a
 * group (and), any group (or). Insights (AI-06) and automations (AUT-01)
 * both edit their conditions here, each offering its own kinds.
 */
import { Button } from '@/ui/button';
import { ICON_PROPS, Plus, X } from '@/ui/icons';
import { type Option, OptionSelect } from '@/ui/option-select';

import { ConditionValue } from './condition-value';
import { conditionsCopy } from './copy';
import { type ConditionOptions, startCondition, useConditionOptions } from './options';
import type { Condition, ConditionKind } from './types';

/** Every kind, in the order the menu lists them. */
export const ALL_CONDITION_KINDS = [
  'title_contains',
  'title_lacks',
  'title_is',
  'title_is_not',
  'title_matches',
  'external',
  'attendee_domain',
  'domains_are_all',
  'domains_not_all',
  'invitee_email',
  'invitee_count',
  'has_participant',
  'lacks_participant',
  'has_tag',
  'lacks_tag',
  'in_folder',
  'template_is',
  'duration_over',
  'calendar_is',
] as const satisfies readonly ConditionKind[];

/** Props of {@link ConditionGroups}. */
export interface ConditionGroupsProps {
  groups: Condition[][];
  onChange: (groups: Condition[][]) => void;
  /** The kinds offered, in menu order; every kind by default. */
  kinds?: readonly ConditionKind[];
}

/** `list` with item `index` replaced by `next`, or removed when `next` is `null`. */
function replaced<T>(list: readonly T[], index: number, next: T | null): T[] {
  return next === null
    ? list.filter((_, at) => at !== index)
    : list.map((item, at) => (at === index ? next : item));
}

/** The groups, each with its conditions, then "Or another group". */
export function ConditionGroups({
  groups,
  onChange,
  kinds = ALL_CONDITION_KINDS,
}: ConditionGroupsProps) {
  const options = useConditionOptions();
  const shown = groups.length === 0 ? [[]] : groups;
  const fresh = startCondition('title_contains', options);
  return (
    <div className="grid gap-2">
      <p className="text-xs text-fg-muted">{conditionsCopy.hint}</p>
      {shown.map((group, index) => (
        <div key={index} className="grid gap-2">
          {index > 0 ? (
            <p className="text-xs font-semibold text-fg-subtle">{conditionsCopy.or}</p>
          ) : null}
          <Group
            group={group}
            menu={{ options, kinds }}
            onChange={(next) => {
              onChange(replaced(shown, index, next));
            }}
          />
        </div>
      ))}
      {fresh === null ? null : (
        <AddButton
          label={conditionsCopy.addGroup}
          onClick={() => {
            onChange([...shown, [fresh]]);
          }}
        />
      )}
    </div>
  );
}

/** What a row's menus offer. */
interface Menu {
  options: ConditionOptions;
  kinds: readonly ConditionKind[];
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
  menu,
  onChange,
}: {
  group: Condition[];
  menu: Menu;
  onChange: (group: Condition[]) => void;
}) {
  const fresh = startCondition('title_contains', menu.options);
  return (
    <div className="grid gap-2">
      {group.map((condition, index) => (
        <ConditionRow
          key={index}
          condition={condition}
          menu={menu}
          onChange={(next) => {
            onChange(replaced(group, index, next));
          }}
        />
      ))}
      {fresh === null ? null : (
        <AddButton
          label={conditionsCopy.addCondition}
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
  menu,
  onChange,
}: {
  condition: Condition;
  menu: Menu;
  onChange: (condition: Condition | null) => void;
}) {
  const kinds: Option<ConditionKind>[] = menu.kinds.map((kind) => ({
    value: kind,
    label: conditionsCopy.kinds[kind],
    disabled: startCondition(kind, menu.options) === null,
  }));
  return (
    <div className="grid grid-cols-[12rem_1fr_auto] items-center gap-2">
      <OptionSelect
        label={conditionsCopy.kinds[condition.kind]}
        value={condition.kind}
        options={kinds}
        onChange={(kind) => {
          const next = startCondition(kind, menu.options);
          if (next !== null) onChange(next);
        }}
      />
      <ConditionValue condition={condition} options={menu.options} onChange={onChange} />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        aria-label={conditionsCopy.removeCondition}
        onClick={() => {
          onChange(null);
        }}
      >
        <X {...ICON_PROPS} />
      </Button>
    </div>
  );
}
