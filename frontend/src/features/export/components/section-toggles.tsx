import { ToggleGroup } from 'radix-ui';

import { cn } from '@/ui/cn';
import { Check, ICON_PROPS } from '@/ui/icons';

import { exportCopy, sectionCopy } from '../copy';
import type { ExportSections, SectionKey } from '../types';

/** The switches shown, in document order. */
export const SECTION_KEYS: readonly SectionKey[] = [
  'attendees',
  'notes',
  'myNotes',
  'actionItems',
  'insights',
  'transcript',
];

/** Props of {@link SectionToggles}. */
export interface SectionTogglesProps {
  value: ExportSections;
  onChange: (sections: ExportSections) => void;
  /** Off for formats whose content is fixed; the chips then explain why. */
  disabled?: boolean;
  /** Accessible name of the group. */
  label?: string;
}

/** `value` with exactly the keys in `chosen` switched on. */
function withChosen(value: ExportSections, chosen: readonly string[]): ExportSections {
  const next = { ...value };
  for (const key of SECTION_KEYS) next[key] = chosen.includes(key);
  return next;
}

/** Which sections to include, as a row of pill toggles. */
export function SectionToggles({ value, onChange, disabled = false, label }: SectionTogglesProps) {
  const chosen = SECTION_KEYS.filter((key) => value[key]);
  return (
    <ToggleGroup.Root
      type="multiple"
      aria-label={label ?? exportCopy.includeLabel}
      value={chosen}
      disabled={disabled}
      onValueChange={(next) => {
        onChange(withChosen(value, next));
      }}
      className="flex flex-wrap gap-1.5"
    >
      {SECTION_KEYS.map((key) => (
        <ToggleGroup.Item
          key={key}
          value={key}
          className={cn(
            'inline-flex h-7 items-center gap-1 rounded-full border border-border px-2.5 text-xs text-fg-muted',
            'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted',
            'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
            'data-[state=on]:border-transparent data-[state=on]:bg-accent-soft data-[state=on]:text-fg',
            'disabled:pointer-events-none disabled:opacity-40',
          )}
        >
          {value[key] ? <Check {...ICON_PROPS} size={12} className="text-accent" /> : null}
          {sectionCopy[key]}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}
