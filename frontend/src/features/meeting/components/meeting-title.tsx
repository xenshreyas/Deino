import { type FormEvent, useState } from 'react';

import { type Meeting, useRenameMeeting } from '@/features/meetings';
import { cn } from '@/ui/cn';

import { meetingCopy } from '../copy';

/** The editing field; mounted per edit so it starts from the current title. */
function TitleField({ meeting, onDone }: { meeting: Meeting; onDone: () => void }) {
  const [draft, setDraft] = useState(meeting.title);
  const rename = useRenameMeeting();
  const save = (event?: FormEvent) => {
    event?.preventDefault();
    const title = draft.trim();
    if (title === '' || title === meeting.title) {
      onDone();
      return;
    }
    rename.mutate({ id: meeting.id, title }, { onSettled: onDone });
  };
  return (
    <form onSubmit={save}>
      <input
        aria-label={meetingCopy.editTitle}
        value={draft}
        onChange={(event) => {
          setDraft(event.target.value);
        }}
        onBlur={() => {
          save();
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') onDone();
        }}
        autoFocus
        className="w-full rounded-sm bg-transparent text-xl font-semibold text-fg outline-none"
      />
    </form>
  );
}

/**
 * The meeting title, 24/600, editable in place (ui.md §3 Meeting header):
 * click to edit, Enter or blur saves, Escape cancels.
 */
export function MeetingTitle({ meeting }: { meeting: Meeting }) {
  const [editing, setEditing] = useState(false);
  if (editing)
    return (
      <TitleField
        meeting={meeting}
        onDone={() => {
          setEditing(false);
        }}
      />
    );
  return (
    <h1 className="text-xl font-semibold text-balance text-fg">
      <button
        type="button"
        title={meetingCopy.editTitle}
        onClick={() => {
          setEditing(true);
        }}
        className={cn(
          'cursor-text rounded-sm text-left decoration-bg-muted decoration-2 underline-offset-[6px]',
          'transition-[text-decoration-color] duration-(--dur-fast) hover:underline hover:decoration-border-strong',
        )}
      >
        {meeting.title}
      </button>
    </h1>
  );
}
