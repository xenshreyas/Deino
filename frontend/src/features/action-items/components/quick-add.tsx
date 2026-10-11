/**
 * Quick add: a quiet row at the top of the list. Type, press Enter, and a
 * standalone item assigned to the user is saved; the field stays ready for
 * the next one. Also where ⌘K's "New action item" lands.
 */
import { type FormEvent, useState } from 'react';

import { IpcError } from '@/lib/ipc/result';
import { cn } from '@/ui/cn';
import { CornerDownLeft, ICON_PROPS, Plus } from '@/ui/icons';
import { Kbd } from '@/ui/kbd';

import { copy } from '../copy';
import { useCreateActionItem } from '../queries';
import type { NewActionItem, PersonId } from '../types';

/** Props of {@link QuickAdd}. */
export interface QuickAddProps {
  /** Who new items are assigned to (the user), if known. */
  assignee: PersonId | null;
  /** Focus the field on mount (⌘K "New action item"). */
  autoFocus?: boolean;
}

/** The title being typed and its save. */
function useQuickAdd(assignee: PersonId | null) {
  const [title, setTitle] = useState('');
  const create = useCreateActionItem();
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (title.trim() === '') return;
    const item: NewActionItem = {
      title,
      description: '',
      meetingId: null,
      assigneePersonId: assignee,
      due: null,
      priority: null,
    };
    create.mutate(item, {
      onSuccess: () => {
        setTitle('');
      },
    });
  };
  const error = create.error === null ? null : IpcError.from(create.error).message;
  return { title, setTitle, submit, error };
}

/** The field and its inline error. */
export function QuickAdd({ assignee, autoFocus = false }: QuickAddProps) {
  const { title, setTitle, submit, error } = useQuickAdd(assignee);
  return (
    <form onSubmit={submit} className="mb-2">
      <label
        className={cn(
          'group flex h-10 items-center gap-3 rounded-md px-3 text-fg-muted transition-colors duration-(--dur-fast)',
          'focus-within:bg-bg-muted hover:bg-bg-muted',
        )}
      >
        <Plus {...ICON_PROPS} className="shrink-0" />
        <span className="sr-only">{copy.quickAdd.label}</span>
        <input
          // ⌘K asked for a new item: take the focus.
          autoFocus={autoFocus}
          value={title}
          placeholder={copy.quickAdd.placeholder}
          onChange={(event) => {
            setTitle(event.target.value);
          }}
          className="min-w-0 flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-muted"
        />
        <span className="flex items-center gap-1.5 text-xs opacity-0 transition-opacity duration-(--dur-fast) group-focus-within:opacity-100">
          <Kbd>
            <CornerDownLeft {...ICON_PROPS} size={12} />
          </Kbd>
          {copy.quickAdd.hint}
        </span>
      </label>
      {error === null ? null : <p className="mt-1 px-3 text-xs text-danger">{error}</p>}
    </form>
  );
}
