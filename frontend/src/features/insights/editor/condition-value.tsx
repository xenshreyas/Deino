/**
 * A condition's value control, by kind: text for a title or a domain, a
 * yes/no for external attendees, a picker of templates, tags or folders.
 */
import { Input } from '@/ui/input';

import { editorCopy } from '../copy';
import type { Condition } from '../types';
import { type ConditionOptions, pointAt, pointing } from './condition-options';
import { type Option, OptionSelect } from './option-select';

/** Props of {@link ConditionValue}. */
export interface ConditionValueProps {
  condition: Condition;
  options: ConditionOptions;
  onChange: (condition: Condition) => void;
}

const AUDIENCE: readonly Option<'yes' | 'no'>[] = [
  { value: 'yes', label: editorCopy.externalYes },
  { value: 'no', label: editorCopy.externalNo },
];

/** The control for `condition`'s kind. */
export function ConditionValue({ condition, options, onChange }: ConditionValueProps) {
  const label = editorCopy.conditions[condition.kind];
  switch (condition.kind) {
    case 'title_contains':
    case 'attendee_domain':
      return <TextValue condition={condition} label={label} onChange={onChange} />;
    case 'external':
      return (
        <OptionSelect
          label={label}
          value={condition.external ? 'yes' : 'no'}
          options={AUDIENCE}
          onChange={(value) => {
            onChange({ ...condition, external: value === 'yes' });
          }}
        />
      );
    default:
      return <PointedValue condition={condition} options={options} onChange={onChange} />;
  }
}

/** The text of a title or domain condition. */
function TextValue({
  condition,
  label,
  onChange,
}: {
  condition: Extract<Condition, { kind: 'title_contains' | 'attendee_domain' }>;
  label: string;
  onChange: (condition: Condition) => void;
}) {
  const isTitle = condition.kind === 'title_contains';
  return (
    <Input
      aria-label={label}
      placeholder={isTitle ? editorCopy.titlePlaceholder : editorCopy.domainPlaceholder}
      value={isTitle ? condition.text : condition.domain}
      onChange={(event) => {
        const value = event.target.value;
        onChange(
          isTitle ? { ...condition, text: value } : { kind: 'attendee_domain', domain: value },
        );
      }}
    />
  );
}

/** A picker of templates, tags or folders. */
function PointedValue({ condition, options, onChange }: ConditionValueProps) {
  const choices = pointing(condition, options);
  if (choices === null) return null;
  return (
    <OptionSelect
      label={editorCopy.conditions[condition.kind]}
      value={choices.value}
      placeholder={editorCopy.choose}
      options={choices.choices.map((choice) => ({ value: choice.id, label: choice.label }))}
      onChange={(id) => {
        onChange(pointAt(condition, id, options));
      }}
    />
  );
}
