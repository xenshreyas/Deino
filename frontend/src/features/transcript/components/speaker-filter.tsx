/**
 * The search bar's speaker filters: the chosen speakers as removable chips
 * inside the field, and a menu of every speaker to add or remove one
 * (the same as typing `speaker:name `).
 */
import { cn } from '@/ui/cn';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Check, ICON_PROPS, Users, X } from '@/ui/icons';
import { Hint } from '@/ui/tooltip';

import { transcriptCopy } from '../copy';
import { speakerTextClass } from '../model/speakers';
import type { SpeakerView } from '../types';

/** A speaker's colour dot (the accent for the user). */
function SpeakerDot({ speaker }: { speaker: SpeakerView }) {
  return (
    <span
      aria-hidden
      className={cn(
        'size-1.5 shrink-0 rounded-full bg-current',
        speaker.isMe ? 'text-accent' : speakerTextClass(speaker.color),
      )}
    />
  );
}

/** Props of {@link SpeakerChips}. */
export interface SpeakerChipsProps {
  chosen: readonly SpeakerView[];
  onRemove: (key: string) => void;
}

/** The chosen speakers, each with a remove button. */
export function SpeakerChips({ chosen, onRemove }: SpeakerChipsProps) {
  return chosen.map((speaker) => (
    <span
      key={speaker.key}
      className="flex h-5 shrink-0 items-center gap-1 rounded-full bg-bg-muted pr-0.5 pl-1.5 text-xs text-fg"
    >
      <SpeakerDot speaker={speaker} />
      <span className="max-w-24 truncate">{speaker.name}</span>
      <button
        type="button"
        aria-label={transcriptCopy.removeSpeaker(speaker.name)}
        onClick={() => {
          onRemove(speaker.key);
        }}
        className="flex size-4 items-center justify-center rounded-full text-fg-muted hover:bg-bg-subtle hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <X {...ICON_PROPS} size={10} />
      </button>
    </span>
  ));
}

/** Props of {@link SpeakerFilterMenu}. */
export interface SpeakerFilterMenuProps {
  speakers: readonly SpeakerView[];
  chosen: readonly string[];
  onToggle: (key: string) => void;
}

/** The "Filter by speaker" button and its menu. */
export function SpeakerFilterMenu({ speakers, chosen, onToggle }: SpeakerFilterMenuProps) {
  return (
    <DropdownMenu>
      <Hint label={transcriptCopy.filterBySpeaker}>
        <DropdownMenuTrigger
          aria-label={transcriptCopy.filterBySpeaker}
          disabled={speakers.length === 0}
          className={cn(
            'flex size-7 shrink-0 items-center justify-center rounded-sm text-fg-muted transition-colors duration-(--dur-fast) ease-out',
            'hover:bg-bg-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus-ring disabled:opacity-40 data-[state=open]:bg-bg-muted',
            chosen.length > 0 && 'text-accent',
          )}
        >
          <Users {...ICON_PROPS} />
        </DropdownMenuTrigger>
      </Hint>
      <DropdownMenuContent align="end">
        {speakers.map((speaker) => (
          <DropdownMenuItem
            key={speaker.key}
            onSelect={() => {
              onToggle(speaker.key);
            }}
          >
            <SpeakerDot speaker={speaker} />
            <span className="truncate">{speaker.name}</span>
            {chosen.includes(speaker.key) ? (
              <Check {...ICON_PROPS} className="ml-auto text-fg" />
            ) : null}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
