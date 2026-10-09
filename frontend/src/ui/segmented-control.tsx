/**
 * Segmented control (ui.md §3 Tabs, "Notes sub-switch"): a pill track with a
 * raised active segment. Used for Enhanced / My notes and the theme picker.
 */
import { ToggleGroup as ToggleGroupPrimitive } from 'radix-ui';

import { cn } from './cn';
import { type Icon, ICON_PROPS } from './icons';

/** One choice in a {@link SegmentedControl}. */
export interface Segment<T extends string> {
  value: T;
  label: string;
  icon?: Icon;
}

/** Props of {@link SegmentedControl}. */
export interface SegmentedControlProps<T extends string> {
  /** Accessible name of the group. */
  label: string;
  segments: readonly Segment<T>[];
  value: T;
  onValueChange: (value: T) => void;
  className?: string;
}

/** A single-choice pill switch; one segment is always selected. */
export function SegmentedControl<T extends string>({
  label,
  segments,
  value,
  onValueChange,
  className,
}: SegmentedControlProps<T>) {
  const choose = (next: string) => {
    const segment = segments.find((candidate) => candidate.value === next);
    if (segment !== undefined) onValueChange(segment.value);
  };
  return (
    <ToggleGroupPrimitive.Root
      type="single"
      aria-label={label}
      value={value}
      onValueChange={choose}
      className={cn(
        'inline-flex h-7 items-center gap-0.5 rounded-full bg-bg-muted p-0.5',
        className,
      )}
    >
      {segments.map((segment) => (
        <SegmentItem key={segment.value} segment={segment} />
      ))}
    </ToggleGroupPrimitive.Root>
  );
}

function SegmentItem<T extends string>({ segment }: { segment: Segment<T> }) {
  const SegmentIcon = segment.icon;
  return (
    <ToggleGroupPrimitive.Item
      value={segment.value}
      className={cn(
        'inline-flex h-6 items-center gap-1.5 rounded-full px-2.5 text-xs text-fg-muted',
        'transition-[background-color,color,box-shadow] duration-(--dur-base) ease-out hover:text-fg',
        'data-[state=on]:bg-surface-raised data-[state=on]:text-fg data-[state=on]:shadow-xs',
      )}
    >
      {SegmentIcon === undefined ? null : <SegmentIcon {...ICON_PROPS} size={14} />}
      {segment.label}
    </ToggleGroupPrimitive.Item>
  );
}
