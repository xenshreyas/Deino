import { RadioGroup } from 'radix-ui';

import { cn } from '@/ui/cn';

import { exportCopy, formatCopy } from '../copy';
import type { ExportFormat } from '../types';

/** The formats, in the order the dialog lists them (`ExportFormat::ALL`). */
export const FORMATS: readonly ExportFormat[] = [
  'markdown',
  'pdf',
  'docx',
  'text',
  'json',
  'srt',
  'vtt',
];

/** Props of {@link FormatList}. */
export interface FormatListProps {
  value: ExportFormat;
  onChange: (format: ExportFormat) => void;
}

/**
 * The format picker: one row per format with what it is for; the chosen
 * row is tinted and marked with the accent. Arrow keys move the choice.
 */
export function FormatList({ value, onChange }: FormatListProps) {
  const choose = (next: string) => {
    const format = FORMATS.find((candidate) => candidate === next);
    if (format !== undefined) onChange(format);
  };
  return (
    <RadioGroup.Root
      aria-label={exportCopy.formatLabel}
      value={value}
      onValueChange={choose}
      className="flex flex-col gap-0.5"
    >
      {FORMATS.map((format) => (
        <RadioGroup.Item
          key={format}
          value={format}
          className={cn(
            'group flex items-start gap-2.5 rounded-sm px-2.5 py-2 text-left outline-none',
            'transition-colors duration-(--dur-fast) ease-out hover:bg-bg-muted',
            'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-focus-ring',
            'data-[state=checked]:bg-accent-soft',
          )}
        >
          <span className="mt-[3px] flex size-3.5 shrink-0 items-center justify-center rounded-full border border-border-strong group-data-[state=checked]:border-accent">
            <RadioGroup.Indicator className="size-1.5 rounded-full bg-accent" />
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="text-sm text-fg">{formatCopy[format].label}</span>
            <span className="truncate text-xs text-fg-muted">{formatCopy[format].hint}</span>
          </span>
        </RadioGroup.Item>
      ))}
    </RadioGroup.Root>
  );
}
