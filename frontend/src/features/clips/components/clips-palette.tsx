/**
 * "Clips" in the ⌘K palette (TRN-04): every clip of every meeting not in
 * the trash, newest first, matched by its name and its meeting's title.
 * Choosing one opens its meeting's transcript at the clip's start.
 */
import { useNavigate } from '@tanstack/react-router';

import { CommandGroup, CommandItem, CommandShortcut } from '@/ui/command';
import { ICON_PROPS, Scissors } from '@/ui/icons';

import { clipsCopy } from '../copy';
import { useAllClips } from '../queries';
import { clipTime } from '../time';

/** How many clips show before the user types. */
const RECENT_LIMIT = 5;

/** What every palette provider receives (the palette's own type, kept structural). */
export interface ClipsPaletteProps {
  search: string;
  close: () => void;
}

/** The provider. */
export function ClipsPalette({ search, close }: ClipsPaletteProps) {
  const navigate = useNavigate();
  const clips = useAllClips().data ?? [];
  if (clips.length === 0) return null;
  const shown = search.trim() === '' ? clips.slice(0, RECENT_LIMIT) : clips;
  return (
    <CommandGroup heading={clipsCopy.paletteHeading}>
      {shown.map((clip) => (
        <CommandItem
          key={clip.id}
          value={`clip ${clip.id} ${clip.name}`}
          keywords={[clip.name, clip.meetingTitle, clipsCopy.paletteKeyword]}
          onSelect={() => {
            close();
            void navigate({
              to: '/meetings/$meetingId',
              params: { meetingId: clip.meetingId },
              search: { tab: 'transcript', t: clip.start },
            });
          }}
        >
          <Scissors {...ICON_PROPS} />
          <span className="truncate">{clip.name}</span>
          <span className="truncate text-fg-muted">{clip.meetingTitle}</span>
          <CommandShortcut>{clipTime(clip.start)}</CommandShortcut>
        </CommandItem>
      ))}
    </CommandGroup>
  );
}
