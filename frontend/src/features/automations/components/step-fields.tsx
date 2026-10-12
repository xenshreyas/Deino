/**
 * One step's settings, by kind: the insight, tag or folder it names, an
 * export's format and folder, a notification's message, a script's command
 * and the payload fields it receives.
 */
import { Button } from '@/ui/button';
import { Folder, ICON_PROPS, TriangleAlert } from '@/ui/icons';
import { Input } from '@/ui/input';
import { type Option, OptionSelect } from '@/ui/option-select';

import { builderCopy, formatCopy, payloadCopy } from '../copy';
import { useChooseFolder } from '../queries';
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
    <div className="grid gap-2 sm:grid-cols-[10rem_minmax(0,1fr)]">
      <OptionSelect
        label={builderCopy.format}
        value={action.format}
        options={FORMATS}
        onChange={(format) => {
          onChange({ ...action, format });
        }}
      />
      <FolderField
        folder={action.folder}
        onPick={(folder) => {
          onChange({ ...action, folder });
        }}
      />
    </div>
  );
}

/** The export folder, chosen in the system folder dialog (never typed). */
function FolderField({ folder, onPick }: { folder: string; onPick: (folder: string) => void }) {
  const choose = useChooseFolder();
  const chosen = folder !== '';
  return (
    <div
      role="group"
      aria-label={builderCopy.exportFolder}
      className="flex min-w-0 items-center gap-2"
    >
      {chosen ? (
        <span className="flex min-w-0 flex-1 items-center gap-1.5 text-sm text-fg" title={folder}>
          <Folder {...ICON_PROPS} className="shrink-0 text-fg-muted" />
          <span className="truncate">{folder}</span>
        </span>
      ) : null}
      <Button
        type="button"
        variant="secondary"
        disabled={choose.isPending}
        onClick={() => {
          choose.mutate(undefined, {
            onSuccess: (picked) => {
              if (picked !== null) onPick(picked);
            },
          });
        }}
      >
        {chosen ? builderCopy.changeFolder : builderCopy.chooseFolder}
      </Button>
      {choose.error === null ? null : (
        <span role="alert" className="text-xs text-danger">
          {choose.error.message}
        </span>
      )}
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
      <FieldPicker
        fields={action.fields}
        onChange={(fields) => {
          onChange({ ...action, fields });
        }}
      />
      {shellAllowed ? null : (
        <p className="flex items-start gap-1.5 text-xs text-warning-text">
          <TriangleAlert {...ICON_PROPS} size={14} className="mt-px shrink-0" />
          {builderCopy.shellOff}
        </p>
      )}
    </div>
  );
}

/** "Payload includes": one checkbox per field a script receives. */
function FieldPicker({
  fields,
  onChange,
}: {
  fields: PayloadFields;
  onChange: (fields: PayloadFields) => void;
}) {
  return (
    <fieldset className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-fg-muted">
      <legend className="sr-only">{builderCopy.payload}</legend>
      <span>{builderCopy.payload}</span>
      {FIELD_KEYS.map((key) => (
        <label key={key} className="flex items-center gap-1.5">
          <input
            type="checkbox"
            className="size-3.5 accent-accent"
            checked={fields[key]}
            onChange={(event) => {
              onChange({ ...fields, [key]: event.target.checked });
            }}
          />
          {payloadCopy[key]}
        </label>
      ))}
    </fieldset>
  );
}
