/**
 * The bulk action bar (Circleback §14 "Bulk actions"): floats at the bottom
 * of the page while meetings are selected, with tag, add to folder, export,
 * merge, run automation (WIP until AUT-01) and delete. A floating layer, so
 * it carries the chat bar's shadow (ui.md §3 Chat bar).
 */
import type { ReactNode } from 'react';

import { Button } from '@/ui/button';
import { Command, CommandEmpty, CommandInput, CommandItem, CommandList } from '@/ui/command';
import { Combine, Download, FolderInput, ICON_PROPS, Tag, Trash2, Workflow, X } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { copy } from '../copy';
import type { FilterOptions } from '../types';

/** Props of {@link BulkBar}. */
export interface BulkBarProps {
  count: number;
  options: FilterOptions;
  onTag: (tagId: FilterOptions['tags'][number]['id']) => void;
  onFile: (folderId: FilterOptions['folders'][number]['id']) => void;
  /** Exports the one selected meeting; `null` when several are selected. */
  onExport: (() => void) | null;
  onMerge: () => void;
  onDelete: () => void;
  onClear: () => void;
}

/** A bar button that opens a searchable list of `items`. */
function PickerButton<Id extends string>(props: {
  icon: ReactNode;
  label: string;
  items: readonly { id: Id; name: string }[];
  onPick: (id: Id) => void;
}) {
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="sm">
          {props.icon}
          {props.label}
        </Button>
      </PopoverTrigger>
      <PopoverContent side="top" className="w-60 p-0">
        <Command>
          <CommandInput placeholder={props.label} aria-label={props.label} />
          <CommandList className="max-h-64">
            <CommandEmpty>{copy.filter.nothing}</CommandEmpty>
            {props.items.map((item) => (
              <CommandItem
                key={item.id}
                value={`${item.name} ${item.id}`}
                onSelect={() => {
                  props.onPick(item.id);
                }}
              >
                {item.name}
              </CommandItem>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

/** "Run automation": shown, but disabled with a WIP badge until AUT-01 (D12). */
function AutomationButton() {
  return (
    <Button variant="ghost" size="sm" disabled aria-disabled="true">
      <Workflow {...ICON_PROPS} />
      {copy.bulk.automation}
      <span className="rounded-full border border-border px-1.5 text-[11px] leading-4 text-fg-muted">
        {copy.bulk.wip}
      </span>
    </Button>
  );
}

/** Tag and Add to folder, each with its picker. */
function PickerButtons({
  options,
  onTag,
  onFile,
}: Pick<BulkBarProps, 'options' | 'onTag' | 'onFile'>) {
  return (
    <>
      <PickerButton
        icon={<Tag {...ICON_PROPS} />}
        label={copy.bulk.tag}
        items={options.tags}
        onPick={onTag}
      />
      <PickerButton
        icon={<FolderInput {...ICON_PROPS} />}
        label={copy.bulk.file}
        items={options.folders}
        onPick={onFile}
      />
    </>
  );
}

/** Export (one meeting at a time) and Merge (two or more). */
function MeetingButtons({
  count,
  onExport,
  onMerge,
}: Pick<BulkBarProps, 'count' | 'onExport' | 'onMerge'>) {
  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        disabled={onExport === null}
        onClick={onExport ?? undefined}
      >
        <Download {...ICON_PROPS} />
        {copy.bulk.export}
      </Button>
      <Button variant="ghost" size="sm" disabled={count < 2} onClick={onMerge}>
        <Combine {...ICON_PROPS} />
        {copy.bulk.merge}
      </Button>
    </>
  );
}

/** The bar, or nothing when nothing is selected. */
export function BulkBar(props: BulkBarProps) {
  if (props.count === 0) return null;
  return (
    <div
      role="toolbar"
      aria-label={copy.bulk.label}
      className="fixed bottom-6 left-1/2 z-(--z-chatbar) flex h-11 -translate-x-1/2 animate-in items-center gap-1 rounded-full bg-surface pr-1.5 pl-4 shadow-chatbar duration-(--dur-base) ease-out fade-in-0 slide-in-from-bottom-2 motion-reduce:animate-none"
    >
      <span className="mr-2 text-sm font-semibold whitespace-nowrap text-fg tabular-nums">
        {copy.bulk.selected(props.count)}
      </span>
      <PickerButtons {...props} />
      <MeetingButtons {...props} />
      <AutomationButton />
      <Button variant="danger" size="sm" onClick={props.onDelete}>
        <Trash2 {...ICON_PROPS} />
        {copy.bulk.delete}
      </Button>
      <span aria-hidden="true" className="mx-1 h-5 w-px bg-border" />
      <IconButton icon={X} label={copy.bulk.clear} shortcut="Esc" onClick={props.onClear} />
    </div>
  );
}
