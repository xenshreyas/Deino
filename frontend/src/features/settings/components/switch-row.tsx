/**
 * One switch with its label and description, as a row of a `SettingsCard`.
 * The settings feature's own sections share it.
 */
import { Switch } from '@/ui/controls';
import { SettingsRow } from '@/ui/settings-section';

/** Props of {@link SwitchRow}. */
export interface SwitchRowProps {
  id: string;
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

/** A labelled switch. */
export function SwitchRow({ id, label, checked, disabled = false, ...props }: SwitchRowProps) {
  return (
    <SettingsRow label={label} htmlFor={id} {...(props.description === undefined ? {} : { description: props.description })}>
      <Switch id={id} checked={checked} disabled={disabled} onCheckedChange={props.onChange} />
    </SettingsRow>
  );
}
