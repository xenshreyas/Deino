/**
 * One step's settings, by kind: the insight, tag or folder it names, an
 * export's format and folder, a notification's message, a script's command
 * and the payload fields it receives.
 */
import { Input } from '@/ui/input';
import { type Option, OptionSelect } from '@/ui/option-select';

import { builderCopy, formatCopy, payloadCopy } from '../copy';
import type { Action, ExportFormat, PayloadFields } from '../types';
import type { StepOptions } from './step-choices';

/** Props of {@link StepFields}. */
export interface StepFieldsProps {
  action: Action;
  options: StepOptions;
  /** Whether shell scripts may run (a script step warns when not). */
  shellAllowed: boolean;
  onChange: (action: Action) => void;
}

const FORMATS: readonly Option<ExportFormat>[] = (
  ['markdown', 'text', 'json', 'srt', 'vtt', 'docx', 'pdf'] as const
).map((value) => ({ value, label: formatCopy[value] }));

const FIELD_KEYS = [
  'notes',
  'actionItems',
  'transcript',
  'insights',
  'attendees',
  'link',
  'audioPath',
] as const satisfies readonly (keyof PayloadFields)[];

/** The controls for `action`'s kind. */
export function StepFields({ action, options, shellAllowed, onChange }: StepFieldsProps) {
  switch (action.kind) {
    case 'generate_insight':
      return (
        <Pick
          label={builderCopy.insight}
          value={action.insightId}
          options={options.insights}
          onPick={(insightId) => {
            onChange({ ...action, insightId });
          }}
        />
      );
    case 'add_tag':
      return (
        <Pick
          label={builderCopy.tag}
          value={action.tagId}
          options={options.tags}
          onPick={(tagId) => {
            onChange({ ...action, tagId });
          }}
        />
      );
    case 'add_to_folder':
      return (
        <Pick
          label={builderCopy.folder}
          value={action.folderId}
          options={options.folders}
          onPick={(folderId) => {
            onChange({ ...action, folderId });
          }}
        />
      );
    default:
      return <LocalFields action={action} shellAllowed={shellAllowed} onChange={onChange} />;
  }
}

/** Export, notification and script settings. */
function LocalFields({ action, shellAllowed, onChange }: Omit<StepFieldsProps, 'options'>) {
  switch (action.kind) {
    case 'export_file':
      return <ExportFields action={action} onChange={onChange} />;
    case 'notify':
      return (
        <Input
          aria-label={builderCopy.message}
          placeholder={builderCopy.message}
          value={action.message}
          onChange={(event) => {
            onChange({ ...action, message: event.target.value });
          }}
        />
      );
    case 'run_shell':
      return <ShellFields action={action} shellAllowed={shellAllowed} onChange={onChange} />;
    default:
      return null;
  }
}

/** A picker of the insight, tag or folder a step names. */
function Pick<T extends string>(props: {
  label: string;
  value: T;
  options: readonly Option<T>[];
  onPick: (value: T) => void;
}) {
  return (
    <OptionSelect
      label={props.label}
      value={props.value}
      options={props.options}
      placeholder={builderCopy.choose}
      onChange={props.onPick}
    />
  );
}

/** An export's format and folder. */
function ExportFields({
  action,
  onChange,
}: {
  action: Extract<Action, { kind: 'export_file' }>;
  onChange: (action: Action) => void;
}) {
  return (
    <div className="grid grid-cols-[10rem_1fr] gap-2">
      <OptionSelect
        label={builderCopy.format}
        value={action.format}
        options={FORMATS}
        onChange={(format) => {
          onChange({ ...action, format });
        }}
      />
      <Input
        aria-label={builderCopy.folder}
        placeholder={builderCopy.exportFolder}
        value={action.folder}
        onChange={(event) => {
          onChange({ ...action, folder: event.target.value });
        }}
      />
    </div>
  );
}

/** A script's command, its payload fields, and a warning while scripts are off. */
function ShellFields({
  action,
  shellAllowed,
  onChange,
}: {
  action: Extract<Action, { kind: 'run_shell' }>;
  shellAllowed: boolean;
  onChange: (action: Action) => void;
}) {
  return (
    <div className="grid gap-2">
      <Input
        aria-label={builderCopy.command}
        placeholder={builderCopy.commandPlaceholder}
        className="font-mono"
        value={action.command}
        onChange={(event) => {
          onChange({ ...action, command: event.target.value });
        }}
      />
      <fieldset className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-fg-muted">
        <legend className="sr-only">{builderCopy.payload}</legend>
        <span>{builderCopy.payload}</span>
        {FIELD_KEYS.map((key) => (
          <label key={key} className="flex items-center gap-1">
            <input
              type="checkbox"
              checked={action.fields[key]}
              onChange={(event) => {
                onChange({ ...action, fields: { ...action.fields, [key]: event.target.checked } });
              }}
            />
            {payloadCopy[key]}
          </label>
        ))}
      </fieldset>
      {shellAllowed ? null : <p className="text-xs text-warning-text">{builderCopy.shellOff}</p>}
    </div>
  );
}
