/**
 * ⌘K results for tags: each tag opens the Meetings page filtered by it.
 * Registered in the shell's `command-palette/providers.ts`.
 */
import { useNavigate } from '@tanstack/react-router';

import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';

import { tagsCopy } from '../copy';
import { useTags } from '../queries';
import { TagDot } from './tag-pill';

/** What every palette provider receives (structurally the shell's props). */
export interface TagsPaletteProps {
  search: string;
  close: () => void;
}

/** "Tags": every tag, shown once the user types. */
export function TagsPalette({ search, close }: TagsPaletteProps) {
  const navigate = useNavigate();
  const tags = useTags().data ?? [];
  if (search.trim() === '' || tags.length === 0) return null;
  return (
    <CommandGroup heading={tagsCopy.palette}>
      {tags.map((tag) => (
        <CommandItem
          key={tag.id}
          value={`tag ${tag.id} ${tag.name}`}
          keywords={[tag.name, tagsCopy.palette]}
          onSelect={() => {
            close();
            void navigate({ to: '/meetings', search: { tags: [tag.id] } });
          }}
        >
          <TagDot color={tag.color} className="mx-[5px] size-2" />
          <span className="truncate">{tagsCopy.paletteShowMeetings(tag.name)}</span>
          <CommandShortcut>{tagsCopy.palette}</CommandShortcut>
        </CommandItem>
      ))}
    </CommandGroup>
  );
}
