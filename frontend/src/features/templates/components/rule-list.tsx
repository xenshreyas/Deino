import type { ReactNode } from 'react';

import { Button } from '@/ui/button';
import { ICON_PROPS, Plus, Trash2 } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Input } from '@/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

import { templatesCopy } from '../copy';
import { type DraftRule, newRule, patchItem } from '../draft';

/** The audience menu's values; `any` is no condition. */
const AUDIENCES = ['any', 'internal', 'external'] as const;

/** Props of {@link RuleList}. */
export interface RuleListProps {
  rules: DraftRule[];
  readOnly: boolean;
  onChange: (rules: DraftRule[]) => void;
}

/** The template's auto rules: any one rule whose conditions all hold picks it. */
export function RuleList({ rules, readOnly, onChange }: RuleListProps) {
  return (
    <section aria-labelledby="template-rules" className="flex flex-col gap-2">
      <div className="flex flex-col gap-0.5">
        <h3 id="template-rules" className="text-sm font-semibold text-fg">
          {templatesCopy.rules}
        </h3>
        <p className="text-xs text-fg-muted">{templatesCopy.rulesHint}</p>
      </div>
      {rules.length === 0 ? null : (
        <ul className="divide-y divide-border rounded-lg border border-border bg-surface">
          {rules.map((rule) => (
            <RuleCard
              key={rule.key}
              rule={rule}
              readOnly={readOnly}
              onChange={(patch) => {
                onChange(patchItem(rules, rule.key, patch));
              }}
              onRemove={() => {
                onChange(rules.filter((candidate) => candidate.key !== rule.key));
              }}
            />
          ))}
        </ul>
      )}
      {readOnly ? null : (
        <AddRule
          onAdd={() => {
            onChange([...rules, newRule()]);
          }}
        />
      )}
    </section>
  );
}

function AddRule({ onAdd }: { onAdd: () => void }) {
  return (
    <Button variant="ghost" size="sm" className="self-start" onClick={onAdd}>
      <Plus {...ICON_PROPS} />
      {templatesCopy.addRule}
    </Button>
  );
}

interface RuleCardProps {
  rule: DraftRule;
  readOnly: boolean;
  onChange: (patch: Partial<DraftRule>) => void;
  onRemove: () => void;
}

/** A text field of the rule. */
type TextField = 'label' | 'titlePattern' | 'domains' | 'min' | 'max';

/** Props binding an input to one text field of the rule. */
function bindText({ rule, readOnly, onChange }: RuleCardProps, field: TextField) {
  return {
    value: rule[field],
    readOnly,
    onChange: (event: { target: { value: string } }) => {
      onChange({ [field]: event.target.value });
    },
  };
}

/** One rule: its label, then its conditions. */
function RuleCard(props: RuleCardProps) {
  return (
    <li className="flex flex-col gap-3 px-3 py-3">
      <div className="flex items-center gap-2">
        <Input
          aria-label={templatesCopy.ruleLabel}
          placeholder={templatesCopy.ruleLabelPlaceholder}
          className="font-semibold"
          {...bindText(props, 'label')}
        />
        {props.readOnly ? null : (
          <IconButton icon={Trash2} label={templatesCopy.removeRule} onClick={props.onRemove} />
        )}
      </div>
      <RuleConditions {...props} />
    </li>
  );
}

/** Title pattern, audience, domains and attendee count, in a 2×2 grid. */
function RuleConditions(props: RuleCardProps) {
  return (
    <div className="grid grid-cols-2 gap-x-3 gap-y-2">
      <Field label={templatesCopy.titlePattern}>
        <Input
          placeholder={templatesCopy.titlePatternPlaceholder}
          className="font-mono text-xs"
          {...bindText(props, 'titlePattern')}
        />
      </Field>
      <Field label={templatesCopy.audience}>
        <AudienceSelect {...props} />
      </Field>
      <Field label={templatesCopy.domains}>
        <Input placeholder={templatesCopy.domainsPlaceholder} {...bindText(props, 'domains')} />
      </Field>
      <Field label={templatesCopy.count}>
        <div className="flex items-center gap-2">
          <Input
            inputMode="numeric"
            aria-label={templatesCopy.min}
            placeholder={templatesCopy.min}
            {...bindText(props, 'min')}
          />
          <span className="text-fg-subtle">–</span>
          <Input
            inputMode="numeric"
            aria-label={templatesCopy.max}
            placeholder={templatesCopy.max}
            {...bindText(props, 'max')}
          />
        </div>
      </Field>
    </div>
  );
}

/** A small label above a control. */
function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs text-fg-muted">{label}</span>
      {children}
    </label>
  );
}

/** Anyone, colleagues only, or someone external. */
function AudienceSelect({ rule, readOnly, onChange }: RuleCardProps) {
  return (
    <Select
      value={rule.audience ?? 'any'}
      disabled={readOnly}
      onValueChange={(value) => {
        onChange({ audience: value === 'internal' || value === 'external' ? value : null });
      }}
    >
      <SelectTrigger aria-label={templatesCopy.audience}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {AUDIENCES.map((audience) => (
          <SelectItem key={audience} value={audience}>
            {audience === 'any' ? templatesCopy.anyAudience : templatesCopy.audiences[audience]}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
