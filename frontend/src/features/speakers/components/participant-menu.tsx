/**
 * A participant's "⋯" menu: rename, or merge into another speaker of the
 * meeting (one voice heard as two). Revealed on row hover and focus.
 */
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { IconButton } from '@/ui/icon-button';
import { Ellipsis, ICON_PROPS, Pencil, Users } from '@/ui/icons';

import { speakersCopy } from '../copy';
import { displayName } from '../format';
import type { Participant, SpeakerId } from '../types';
import { SpeakerAvatar } from './speaker-avatar';

/** Props of {@link ParticipantMenu}. */
export interface ParticipantMenuProps {
  participant: Participant;
  /** The speakers it could merge into. */
  others: readonly Participant[];
  onRename: () => void;
  onMerge: (into: SpeakerId) => void;
}

/** The merge targets, as a submenu; absent for the user's own voice. */
function MergeSubmenu({
  others,
  onMerge,
}: {
  others: readonly Participant[];
  onMerge: (into: SpeakerId) => void;
}) {
  return (
    <DropdownMenuSub>
      <DropdownMenuSubTrigger>
        <Users {...ICON_PROPS} />
        {speakersCopy.mergeInto}
      </DropdownMenuSubTrigger>
      <DropdownMenuSubContent>
        {others.map((other) => (
          <DropdownMenuItem
            key={other.speakerId}
            onSelect={() => {
              onMerge(other.speakerId);
            }}
          >
            <SpeakerAvatar
              name={displayName(other)}
              color={other.color}
              unnamed={other.person === null && !other.isMe}
              size={14}
            />
            <span className="truncate">{displayName(other)}</span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuSubContent>
    </DropdownMenuSub>
  );
}

/** The hover-revealed menu of one row. */
export function ParticipantMenu({ participant, others, onRename, onMerge }: ParticipantMenuProps) {
  const canMerge = !participant.isMe && others.length > 0;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <IconButton
          icon={Ellipsis}
          label={speakersCopy.rowActions(displayName(participant))}
          className="opacity-0 group-focus-within/row:opacity-100 group-hover/row:opacity-100 data-[state=open]:opacity-100"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={onRename}>
          <Pencil {...ICON_PROPS} />
          {speakersCopy.rename}
        </DropdownMenuItem>
        {canMerge ? <MergeSubmenu others={others} onMerge={onMerge} /> : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
