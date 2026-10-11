/**
 * The person and company forms' shared parts: a labelled text field, the
 * error line, and Cancel plus the submit button.
 */
import { Button } from '@/ui/button';
import { Label } from '@/ui/controls';
import { DialogFooter } from '@/ui/dialog';
import { Input } from '@/ui/input';

import { entitiesCopy } from '../copy';

/** Props of {@link TextField}. */
export interface TextFieldProps {
  id: string;
  label: string;
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  type?: 'text' | 'email';
  autoFocus?: boolean;
  invalid?: boolean;
}

/** One labelled single-line field. */
export function TextField(props: TextFieldProps) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={props.id}>{props.label}</Label>
      <Input
        id={props.id}
        type={props.type ?? 'text'}
        value={props.value}
        placeholder={props.placeholder}
        autoFocus={props.autoFocus}
        aria-invalid={props.invalid === true}
        onChange={(event) => {
          props.onChange(event.target.value);
        }}
      />
    </div>
  );
}

/** What Rust refused, in its own words. */
export function FormError({ error }: { error: Error | null }) {
  return error === null ? null : <p className="text-xs text-danger">{error.message}</p>;
}

/** Cancel and Create (or Save). */
export function FormFooter({
  creating,
  disabled,
  onClose,
}: {
  creating: boolean;
  disabled: boolean;
  onClose: () => void;
}) {
  return (
    <DialogFooter>
      <Button variant="ghost" type="button" onClick={onClose}>
        {entitiesCopy.cancel}
      </Button>
      <Button type="submit" disabled={disabled}>
        {creating ? entitiesCopy.create : entitiesCopy.save}
      </Button>
    </DialogFooter>
  );
}
