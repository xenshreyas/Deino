/**
 * The Meetings page's tag filter (ui.md §3 Page header: filter chips): a
 * 28px chip that opens the tag list; active when any tag is chosen.
 */
import { cn } from '@/ui/cn';
import { ChevronDown, ICON_PROPS, Tag as TagIcon, X } from '@/ui/icons';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { tagsCopy } from '../copy';
import { useTags } from '../queries';
import type { Tag, TagId } from '../types';
import { TagDot } from './tag-pill';
import { TagPicker } from './tag-picker';

/** Props of {@link TagFilter}. */
export interface TagFilterProps {
  selected: readonly TagId[];
  onChange: (selected: TagId[]) => void;
}

/** What the chip says: "Tag", the one tag, or the first and a count. */
function ChipLabel({ chosen }: { chosen: readonly Tag[] }) {
  const [first] = chosen;
  if (first === undefined) {
    return (
      <>
        <TagIcon {...ICON_PROPS} size={14} />
        {tagsCopy.filterLabel}
      </>
    );
  }
  return (
    <>
      <TagDot color={first.color} />
      <span className="max-w-32 truncate">{first.name}</span>
      {chosen.length > 1 ? <span className="tabular-nums">+{chosen.length - 1}</span> : null}
    </>
  );
}

/** Clears the filter. */
function ClearButton({ onClear }: { onClear: () => void }) {
  return (
    <button
      type="button"
      aria-label={tagsCopy.filterClear}
      onClick={onClear}
      className="flex size-6 items-center justify-center rounded-full text-fg-subtle transition-colors duration-(--dur-fast) hover:bg-bg-muted hover:text-fg"
    >
      <X {...ICON_PROPS} size={14} />
    </button>
  );
}

/** A filter chip over the tag list; choosing toggles a tag in the filter. */
export function TagFilter({ selected, onChange }: TagFilterProps) {
  const tags = useTags().data ?? [];
  const chosen = tags.filter((tag) => selected.includes(tag.id));
  const active = chosen.length > 0;
  const toggle = (tag: Tag) => {
    const without = selected.filter((id) => id !== tag.id);
    onChange(without.length < selected.length ? without : [...selected, tag.id]);
  };
  return (
    <div className="flex items-center gap-1">
      <Popover>
        <PopoverTrigger
          className={cn(
            'inline-flex h-7 items-center gap-1.5 rounded-full border border-border px-2.5 text-xs transition-colors duration-(--dur-fast)',
            active ? 'bg-accent-soft text-fg' : 'text-fg-muted hover:bg-bg-muted hover:text-fg',
          )}
        >
          <ChipLabel chosen={chosen} />
          <ChevronDown {...ICON_PROPS} size={14} />
        </PopoverTrigger>
        <PopoverContent align="start" className="w-60 p-0">
          <TagPicker
            tags={tags}
            selected={new Set(selected)}
            onToggle={toggle}
            placeholder={tagsCopy.filterLabel}
          />
        </PopoverContent>
      </Popover>
      {active ? (
        <ClearButton
          onClear={() => {
            onChange([]);
          }}
        />
      ) : null}
    </div>
  );
}
