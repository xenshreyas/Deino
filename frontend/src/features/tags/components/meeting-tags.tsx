/**
 * A meeting's tags in its header (ui.md §3 Meeting header): a pill per tag,
 * then a ghost "Add tag" chip that opens the tag combobox. Tagging by hand
 * keeps the auto-tagger away from the meeting (ORG-06).
 */
import { useState } from 'react';
import { toast } from 'sonner';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import { Chip } from '@/ui/chip';
import { Plus } from '@/ui/icons';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';

import { tagsCopy } from '../copy';
import {
  useApplyTag,
  useCreateAndApplyTag,
  useMeetingTags,
  useRemoveTag,
  useTags,
} from '../queries';
import type { Tag } from '../types';
import { TagPicker } from './tag-picker';
import { TagPill } from './tag-pill';

/** Reports a failed tag change; the message is user-safe (IPC errors are). */
function reportFailure(error: Error) {
  toast.error(tagsCopy.applyFailed, { description: error.message });
}

/** Applies, removes and creates the meeting's tags. */
function useMeetingTagActions(meetingId: MeetingId) {
  const apply = useApplyTag();
  const remove = useRemoveTag();
  const create = useCreateAndApplyTag();
  const options = { onError: reportFailure };
  return {
    add: (tag: Tag) => {
      apply.mutate({ meetingId, tagId: tag.id }, options);
    },
    remove: (tag: Tag) => {
      remove.mutate({ meetingId, tagId: tag.id }, options);
    },
    create: (name: string) => {
      create.mutate({ meetingId, name }, options);
    },
  };
}

/** The "Add tag" chip and its combobox. */
function AddTagChip({
  meetingId,
  applied,
  className,
}: {
  meetingId: MeetingId;
  applied: readonly Tag[];
  className?: string | undefined;
}) {
  const [open, setOpen] = useState(false);
  const tags = useTags();
  const actions = useMeetingTagActions(meetingId);
  const selected = new Set(applied.map((tag) => tag.id));
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Chip variant="ghost" icon={Plus} className={className}>
          {tagsCopy.addTag}
        </Chip>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-64 p-0">
        <TagPicker
          tags={tags.data ?? []}
          selected={selected}
          onToggle={(tag) => {
            if (selected.has(tag.id)) actions.remove(tag);
            else actions.add(tag);
          }}
          onCreate={actions.create}
        />
      </PopoverContent>
    </Popover>
  );
}

/** Props of {@link MeetingTags}. */
export interface MeetingChipsProps {
  meetingId: MeetingId;
  /** Classes for the ghost "Add tag" chip (the header reveals it on hover). */
  addChipClassName?: string | undefined;
}

/** The meeting's tag pills and the "Add tag" chip. */
export function MeetingTags({ meetingId, addChipClassName }: MeetingChipsProps) {
  const applied = useMeetingTags(meetingId);
  const actions = useMeetingTagActions(meetingId);
  const tags = applied.data ?? [];
  return (
    <>
      {tags.map(({ tag, appliedBy }) => (
        <TagPill
          key={tag.id}
          tag={tag}
          byDeino={appliedBy === 'ai'}
          onRemove={() => {
            actions.remove(tag);
          }}
        />
      ))}
      <AddTagChip
        meetingId={meetingId}
        applied={tags.map(({ tag }) => tag)}
        className={addChipClassName}
      />
    </>
  );
}
