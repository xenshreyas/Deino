import { useId } from 'react';

import { eventCopy, fieldCopy, webhooksCopy } from '../copy';
import {
  PAYLOAD_FIELD_KEYS,
  type PayloadFields,
  WEBHOOK_EVENTS,
  type WebhookEvent,
} from '../types';

/** Props of {@link CheckboxGroup}. */
interface CheckboxGroupProps<T extends string> {
  legend: string;
  options: readonly T[];
  labels: Record<T, string>;
  isChecked: (option: T) => boolean;
  onToggle: (option: T, checked: boolean) => void;
}

/** A legend and a wrapping row of small checkboxes. */
function CheckboxGroup<T extends string>(props: CheckboxGroupProps<T>) {
  const id = useId();
  return (
    <fieldset className="flex flex-col gap-1.5" aria-labelledby={id}>
      <span id={id} className="text-xs text-fg-muted">
        {props.legend}
      </span>
      <div className="flex flex-wrap gap-x-4 gap-y-1.5">
        {props.options.map((option) => (
          <label key={option} className="flex items-center gap-1.5 text-sm text-fg">
            <input
              type="checkbox"
              className="size-3.5 accent-accent"
              checked={props.isChecked(option)}
              onChange={(event) => {
                props.onToggle(option, event.target.checked);
              }}
            />
            {props.labels[option]}
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/** Props of {@link FieldPicker}. */
export interface FieldPickerProps {
  fields: PayloadFields;
  onChange: (fields: PayloadFields) => void;
}

/** "Payload includes": one checkbox per part of the meeting a webhook sends. */
export function FieldPicker({ fields, onChange }: FieldPickerProps) {
  return (
    <CheckboxGroup
      legend={webhooksCopy.payload}
      options={PAYLOAD_FIELD_KEYS}
      labels={fieldCopy}
      isChecked={(key) => fields[key]}
      onToggle={(key, checked) => {
        onChange({ ...fields, [key]: checked });
      }}
    />
  );
}

/** Props of {@link EventPicker}. */
export interface EventPickerProps {
  events: readonly WebhookEvent[];
  onChange: (events: WebhookEvent[]) => void;
}

/** "Send when": one checkbox per event. */
export function EventPicker({ events, onChange }: EventPickerProps) {
  return (
    <CheckboxGroup
      legend={webhooksCopy.events}
      options={WEBHOOK_EVENTS}
      labels={eventCopy}
      isChecked={(event) => events.includes(event)}
      onToggle={(event, checked) => {
        const others = events.filter((other) => other !== event);
        onChange(
          checked ? WEBHOOK_EVENTS.filter((e) => e === event || others.includes(e)) : others,
        );
      }}
    />
  );
}
