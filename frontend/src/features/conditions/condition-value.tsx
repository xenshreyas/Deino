/**
 * A condition's value control, by kind: text for titles, patterns,
 * domains and emails; a comma-separated list for domain sets; a number
 * for counts and minutes; yes/no for external attendees; a picker of
 * templates, tags, folders, people or calendars.
 */
import { Input } from '@/ui/input';
import { type Option, OptionSelect } from '@/ui/option-select';

import { conditionsCopy } from './copy';
import { type ConditionOptions, pointAt, pointing } from './options';
import type { Comparison, Condition } from './types';

/** Props of {@link ConditionValue}. */
export interface ConditionValueProps {
  condition: Condition;
  options: ConditionOptions;
  onChange: (condition: Condition) => void;
}

const AUDIENCE: readonly Option<'yes' | 'no'>[] = [
  { value: 'yes', label: conditionsCopy.externalYes },
  { value: 'no', label: conditionsCopy.externalNo },
];

const COMPARISONS: readonly Option<Comparison>[] = (['equal', 'greater', 'less'] as const).map(
  (value) => ({ value, label: conditionsCopy.comparisons[value] }),
);

/** `"acme.com, , acme.io"` as `["acme.com", "acme.io"]`. */
export function splitList(line: string): string[] {
  return line
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item !== '');
}

/** A non-negative whole number from an input, or `fallback`. */
function wholeNumber(raw: string, fallback: number): number {
  const value = Number.parseInt(raw, 10);
  return Number.isFinite(value) && value >= 0 ? value : fallback;
}

/** The control for `condition`'s kind. */
export function ConditionValue({ condition, options, onChange }: ConditionValueProps) {
  const label = conditionsCopy.kinds[condition.kind];
  switch (condition.kind) {
    case 'title_contains':
    case 'title_lacks':
    case 'title_is':
    case 'title_is_not':
      return (
        <Text
          label={label}
          value={condition.text}
          placeholder={conditionsCopy.placeholders.title}
          onChange={(text) => {
            onChange({ ...condition, text });
          }}
        />
      );
    case 'title_matches':
      return (
        <Text
          label={label}
          value={condition.pattern}
          placeholder={conditionsCopy.placeholders.pattern}
          onChange={(pattern) => {
            onChange({ ...condition, pattern });
          }}
        />
      );
    default:
      return <PeopleValue condition={condition} options={options} onChange={onChange} />;
  }
}

/** Controls about who attended; pickers for the rest. */
function PeopleValue({ condition, options, onChange }: ConditionValueProps) {
  const label = conditionsCopy.kinds[condition.kind];
  switch (condition.kind) {
    case 'attendee_domain':
      return (
        <Text
          label={label}
          value={condition.domain}
          placeholder={conditionsCopy.placeholders.domain}
          onChange={(domain) => {
            onChange({ ...condition, domain });
          }}
        />
      );
    case 'domains_are_all':
    case 'domains_not_all':
      return <DomainsValue condition={condition} label={label} onChange={onChange} />;
    case 'invitee_email':
      return <EmailValue condition={condition} label={label} onChange={onChange} />;
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
      return <NumberValue condition={condition} options={options} onChange={onChange} />;
  }
}

/** Counts and minutes; pickers for the rest. */
function NumberValue({ condition, options, onChange }: ConditionValueProps) {
  if (condition.kind === 'duration_over') {
    return (
      <div className="flex items-center gap-2">
        <Count
          label={conditionsCopy.kinds.duration_over}
          value={condition.minutes}
          onChange={(minutes) => {
            onChange({ ...condition, minutes });
          }}
        />
        <span className="text-sm text-fg-muted">{conditionsCopy.minutes}</span>
      </div>
    );
  }
  if (condition.kind === 'invitee_count') {
    return (
      <div className="grid grid-cols-[1fr_5rem] gap-2">
        <OptionSelect
          label={conditionsCopy.kinds.invitee_count}
          value={condition.comparison}
          options={COMPARISONS}
          onChange={(comparison) => {
            onChange({ ...condition, comparison });
          }}
        />
        <Count
          label={conditionsCopy.count}
          value={condition.count}
          onChange={(count) => {
            onChange({ ...condition, count });
          }}
        />
      </div>
    );
  }
  return <PointedValue condition={condition} options={options} onChange={onChange} />;
}

/** One line of text. */
function Text(props: {
  label: string;
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
}) {
  return (
    <Input
      aria-label={props.label}
      placeholder={props.placeholder}
      value={props.value}
      onChange={(event) => {
        props.onChange(event.target.value);
      }}
    />
  );
}

/** A whole number. */
function Count(props: { label: string; value: number; onChange: (value: number) => void }) {
  return (
    <Input
      aria-label={props.label}
      type="number"
      min={0}
      value={props.value}
      onChange={(event) => {
        props.onChange(wholeNumber(event.target.value, props.value));
      }}
    />
  );
}

/** A comma-separated list of domains. */
function DomainsValue({
  condition,
  label,
  onChange,
}: {
  condition: Extract<Condition, { kind: 'domains_are_all' | 'domains_not_all' }>;
  label: string;
  onChange: (condition: Condition) => void;
}) {
  return (
    <Text
      label={label}
      value={condition.domains.join(', ')}
      placeholder={conditionsCopy.placeholders.domains}
      onChange={(line) => {
        onChange({ ...condition, domains: splitList(line) });
      }}
    />
  );
}

/** An invitee's address; Rust checks it when the automation is saved. */
function EmailValue({
  condition,
  label,
  onChange,
}: {
  condition: Extract<Condition, { kind: 'invitee_email' }>;
  label: string;
  onChange: (condition: Condition) => void;
}) {
  return (
    <Text
      label={label}
      value={condition.email}
      placeholder={conditionsCopy.placeholders.email}
      onChange={(email) => {
        onChange({ ...condition, email: email.trim().toLowerCase() });
      }}
    />
  );
}

/** A picker of templates, tags, folders, people or calendars. */
function PointedValue({ condition, options, onChange }: ConditionValueProps) {
  const choices = pointing(condition, options);
  if (choices === null) return null;
  return (
    <OptionSelect
      label={conditionsCopy.kinds[condition.kind]}
      value={choices.value}
      placeholder={conditionsCopy.choose}
      options={choices.choices.map((choice) => ({ value: choice.id, label: choice.label }))}
      onChange={(id) => {
        onChange(pointAt(condition, id, options));
      }}
    />
  );
}
