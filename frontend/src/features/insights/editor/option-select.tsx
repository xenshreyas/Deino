/**
 * A select over a fixed list of typed options: the editor's field types,
 * condition kinds and condition values all pick from one.
 */
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

/** One choice. */
export interface Option<T extends string> {
  value: T;
  label: string;
  disabled?: boolean;
}

/** Props of {@link OptionSelect}. */
export interface OptionSelectProps<T extends string> {
  /** The accessible name. */
  label: string;
  value: T;
  options: readonly Option<T>[];
  onChange: (value: T) => void;
  placeholder?: string;
}

/** The select; only values from `options` reach `onChange`. */
export function OptionSelect<T extends string>({
  label,
  value,
  options,
  onChange,
  placeholder,
}: OptionSelectProps<T>) {
  return (
    <Select
      value={value}
      onValueChange={(next) => {
        const option = options.find((candidate) => candidate.value === next);
        if (option !== undefined) onChange(option.value);
      }}
    >
      <SelectTrigger aria-label={label}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value} disabled={option.disabled ?? false}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
