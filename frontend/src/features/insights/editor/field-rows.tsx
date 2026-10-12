/**
 * The editor's typed fields: one row each (name, type, and the options of
 * a "one of a list" field), plus "Add field". No fields means each answer
 * is plain text.
 */
import { Button } from '@/ui/button';
import { ICON_PROPS, Plus, X } from '@/ui/icons';
import { Input } from '@/ui/input';

import { editorCopy } from '../copy';
import type { FieldKind } from '../types';
import { emptyField, type FieldDraft } from './draft';
import { type Option, OptionSelect } from '@/ui/option-select';

/** Props of {@link FieldRows}. */
export interface FieldRowsProps {
  fields: FieldDraft[];
  onChange: (fields: FieldDraft[]) => void;
}

/** The field types, in the order the menu lists them. */
const TYPES: readonly Option<FieldKind['type']>[] = (
  ['text', 'number', 'boolean', 'date', 'enum'] as const
).map((type) => ({ value: type, label: editorCopy.types[type] }));

/** The rows and the add button. */
export function FieldRows({ fields, onChange }: FieldRowsProps) {
  const replace = (key: string, next: FieldDraft | null) => {
    onChange(
      next === null
        ? fields.filter((field) => field.key !== key)
        : fields.map((field) => (field.key === key ? next : field)),
    );
  };
  return (
    <fieldset className="grid gap-2">
      <legend className="text-sm font-semibold text-fg">{editorCopy.fields}</legend>
      <p className="-mt-1 text-xs text-fg-muted">{editorCopy.fieldsHint}</p>
      {fields.map((field) => (
        <FieldRow
          key={field.key}
          field={field}
          onChange={(next) => {
            replace(field.key, next);
          }}
        />
      ))}
      <div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => {
            onChange([...fields, emptyField()]);
          }}
        >
          <Plus {...ICON_PROPS} />
          {editorCopy.addField}
        </Button>
      </div>
    </fieldset>
  );
}

/** One field: name, type, remove, and the options of an enum. */
function FieldRow({
  field,
  onChange,
}: {
  field: FieldDraft;
  /** The edited field, or `null` to remove it. */
  onChange: (field: FieldDraft | null) => void;
}) {
  const set = (change: Partial<FieldDraft>) => {
    onChange({ ...field, ...change });
  };
  return (
    <div className="grid grid-cols-[1fr_9.5rem_auto] items-start gap-2">
      <NameInput field={field} onChange={set} />
      <OptionSelect
        label={editorCopy.fieldType}
        value={field.type}
        options={TYPES}
        onChange={(type) => {
          set({ type });
        }}
      />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        aria-label={editorCopy.removeField(field.name)}
        onClick={() => {
          onChange(null);
        }}
        className="mt-0.5"
      >
        <X {...ICON_PROPS} />
      </Button>
      {field.type === 'enum' ? <OptionsInput field={field} onChange={set} /> : null}
    </div>
  );
}

/** A field's name. */
function NameInput({
  field,
  onChange,
}: {
  field: FieldDraft;
  onChange: (change: Partial<FieldDraft>) => void;
}) {
  return (
    <Input
      aria-label={editorCopy.fieldName}
      placeholder={editorCopy.fieldName}
      value={field.name}
      maxLength={60}
      onChange={(event) => {
        onChange({ name: event.target.value });
      }}
    />
  );
}

/** An enum field's options, comma-separated. */
function OptionsInput({
  field,
  onChange,
}: {
  field: FieldDraft;
  onChange: (change: Partial<FieldDraft>) => void;
}) {
  return (
    <Input
      aria-label={editorCopy.options}
      placeholder={editorCopy.optionsPlaceholder}
      value={field.options}
      onChange={(event) => {
        onChange({ options: event.target.value });
      }}
      className="col-span-2"
    />
  );
}
