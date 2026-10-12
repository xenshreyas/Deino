/**
 * The bulk bar's Export: one selected meeting opens the export dialog (with
 * its preview and section switches); several open a menu of formats, and
 * the pick writes one file per meeting into a folder Rust asks for.
 */
import { type ExportFormat, FORMATS, formatCopy } from '@/features/export';
import { Button } from '@/ui/button';
import { Download, ICON_PROPS } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { copy } from '../copy';

/** Props of {@link ExportButton}. */
export interface ExportButtonProps {
  /** Opens the export dialog; set when exactly one meeting is selected. */
  onExportOne: (() => void) | null;
  /** Exports every selected meeting as `format`. */
  onExportMany: (format: ExportFormat) => void;
}

const LABEL = (
  <>
    <Download {...ICON_PROPS} />
    {copy.bulk.export}
  </>
);

/** Export, as a button for one meeting or a format menu for many. */
export function ExportButton({ onExportOne, onExportMany }: ExportButtonProps) {
  if (onExportOne !== null) {
    return (
      <Button variant="ghost" size="sm" onClick={onExportOne}>
        {LABEL}
      </Button>
    );
  }
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm">
          {LABEL}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" className="w-72">
        <DropdownMenuLabel>{copy.bulk.exportAs}</DropdownMenuLabel>
        {FORMATS.map((format) => (
          <DropdownMenuItem
            key={format}
            onSelect={() => {
              onExportMany(format);
            }}
          >
            <span className="flex-1">{formatCopy[format].label}</span>
            <span className="truncate text-xs text-fg-subtle">{formatCopy[format].hint}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
