/**
 * A searchable folder list for popovers and dialogs (cmdk): folders indent
 * by depth and show their icon; chosen ones carry a check. Used to add a
 * meeting to folders and to pick where a folder moves.
 */
import type { ReactNode } from 'react';

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/ui/command';
import { Check, ICON_PROPS } from '@/ui/icons';

import { foldersCopy } from '../copy';
import { folderIcon } from '../folder-icons';
import type { FolderId, FolderTreeEntry } from '../types';

/** Props of {@link FolderPicker}. */
export interface FolderPickerProps {
  entries: readonly FolderTreeEntry[];
  selected?: ReadonlySet<FolderId>;
  onSelect: (entry: FolderTreeEntry) => void;
  placeholder?: string;
  empty?: string;
  /** Extra rows above the folders ("Top level", "New folder…"). */
  children?: ReactNode;
}

/** Indentation per tree level, in px (ui.md §3 Sidebar: 12px a level). */
const INDENT_PX = 12;

/** One folder row. */
function FolderOption({
  entry,
  checked,
  onSelect,
}: {
  entry: FolderTreeEntry;
  checked: boolean;
  onSelect: () => void;
}) {
  const FolderIcon = folderIcon(entry.folder.icon);
  return (
    <CommandItem
      value={`folder ${entry.folder.id} ${entry.folder.name}`}
      keywords={[entry.folder.name]}
      onSelect={onSelect}
      style={{ paddingLeft: 10 + entry.depth * INDENT_PX }}
    >
      <FolderIcon {...ICON_PROPS} />
      <span className="min-w-0 flex-1 truncate">{entry.folder.name}</span>
      {checked ? <Check {...ICON_PROPS} className="text-fg" /> : null}
    </CommandItem>
  );
}

/** The list; selecting a row calls `onSelect` and leaves closing to the caller. */
export function FolderPicker({
  entries,
  selected,
  onSelect,
  placeholder,
  empty,
  children,
}: FolderPickerProps) {
  return (
    <Command loop>
      <CommandInput placeholder={placeholder ?? foldersCopy.moveSearch} />
      <CommandList className="max-h-72">
        <CommandEmpty>{empty ?? foldersCopy.noTargets}</CommandEmpty>
        <CommandGroup>
          {children}
          {entries.map((entry) => (
            <FolderOption
              key={entry.folder.id}
              entry={entry}
              checked={selected?.has(entry.folder.id) ?? false}
              onSelect={() => {
                onSelect(entry);
              }}
            />
          ))}
        </CommandGroup>
      </CommandList>
    </Command>
  );
}
