/**
 * The tag combobox's list: search, toggle tags with a check, and create a
 * new tag from the search (ui.md §3 Popover; cmdk does the keyboard work).
 * Presentational: the caller owns the data and the mutations.
 */
import { useState } from 'react';

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/ui/command';
import { Check, ICON_PROPS, Plus } from '@/ui/icons';

import { tagsCopy } from '../copy';
import type { Tag, TagId } from '../types';
import { TagDot } from './tag-pill';

/** Props of {@link TagPicker}. */
export interface TagPickerProps {
  tags: readonly Tag[];
  selected: ReadonlySet<TagId>;
  onToggle: (tag: Tag) => void;
  /** Creates a tag named after the search; omit to allow existing tags only. */
  onCreate?: ((name: string) => void) | undefined;
  placeholder?: string;
}

/** True when `name` is the name of one of `tags`, ignoring case and spaces. */
export function hasTagNamed(tags: readonly Tag[], name: string): boolean {
  const wanted = name.trim().toLowerCase();
  return tags.some((tag) => tag.name.toLowerCase() === wanted);
}

/** One tag: dot, name, and a check when chosen. */
function TagOption({
  tag,
  checked,
  onToggle,
}: {
  tag: Tag;
  checked: boolean;
  onToggle: (tag: Tag) => void;
}) {
  return (
    <CommandItem
      value={`tag ${tag.id} ${tag.name}`}
      keywords={[tag.name]}
      onSelect={() => {
        onToggle(tag);
      }}
    >
      <TagDot color={tag.color} className="mx-[5px]" />
      <span className="min-w-0 flex-1 truncate">{tag.name}</span>
      {checked ? <Check {...ICON_PROPS} className="text-fg" /> : null}
    </CommandItem>
  );
}

/** "Create “name”", the last row while the search names no tag. */
function CreateOption({ name, onCreate }: { name: string; onCreate: () => void }) {
  return (
    <CommandItem value={`create ${name}`} keywords={[name]} onSelect={onCreate}>
      <Plus {...ICON_PROPS} />
      <span className="truncate">{tagsCopy.createNamed(name)}</span>
    </CommandItem>
  );
}

/** A searchable, checkable tag list with an optional "Create …" row. */
export function TagPicker({ tags, selected, onToggle, onCreate, placeholder }: TagPickerProps) {
  const [search, setSearch] = useState('');
  const name = search.trim();
  const create =
    onCreate === undefined || name === '' || hasTagNamed(tags, name)
      ? null
      : () => {
          onCreate(name);
          setSearch('');
        };
  return (
    <Command loop>
      <CommandInput
        value={search}
        onValueChange={setSearch}
        placeholder={placeholder ?? tagsCopy.searchPlaceholder}
      />
      <CommandList className="max-h-72">
        <CommandEmpty>{tagsCopy.noTags}</CommandEmpty>
        <CommandGroup>
          {tags.map((tag) => (
            <TagOption key={tag.id} tag={tag} checked={selected.has(tag.id)} onToggle={onToggle} />
          ))}
          {create === null ? null : <CreateOption name={name} onCreate={create} />}
        </CommandGroup>
      </CommandList>
    </Command>
  );
}
