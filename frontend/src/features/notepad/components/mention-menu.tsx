/**
 * The @mention menu: a popover under the typed `@query` (ui.md §3 Popover),
 * keyboard-driven by the editor (↑ ↓ ↩ ⇥ ⎋) and clickable. It never takes
 * focus from the editor.
 */
import { createPortal } from 'react-dom';
import { useStore } from 'zustand';

import { cn } from '@/ui/cn';
import { ICON_PROPS, Plus } from '@/ui/icons';

import { notepadCopy } from '../copy';
import { initials } from '../editor/mention';
import type { MentionStore } from '../editor/menu-store';
import type { MentionCandidate } from '../types';

/** Gap between the typed query and the menu, in px. */
const MENU_OFFSET = 6;

/** Initials, or a dashed "+" for someone not added yet. */
function OptionAvatar({ candidate }: { candidate: MentionCandidate }) {
  const isNew = candidate.isNew === true;
  return (
    <span
      aria-hidden="true"
      className={cn(
        'flex size-5 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold',
        isNew
          ? 'border border-dashed border-border-strong text-fg-muted'
          : 'bg-accent-soft text-accent',
      )}
    >
      {isNew ? <Plus {...ICON_PROPS} size={12} /> : initials(candidate.label)}
    </span>
  );
}

/** One person in the menu. */
function MentionOption({
  candidate,
  active,
  onChoose,
}: {
  candidate: MentionCandidate;
  active: boolean;
  onChoose: () => void;
}) {
  return (
    <li
      role="option"
      aria-selected={active}
      onMouseDown={(event) => {
        event.preventDefault();
        onChoose();
      }}
      className={cn(
        'flex h-[30px] cursor-default items-center gap-2 rounded-sm px-2 text-sm text-fg',
        active && 'bg-bg-muted',
      )}
    >
      <OptionAvatar candidate={candidate} />
      <span className="truncate">
        {candidate.isNew === true ? notepadCopy.addPerson(candidate.label) : candidate.label}
      </span>
      {candidate.detail === null ? null : (
        <span className="ml-auto truncate pl-2 text-xs text-fg-subtle">{candidate.detail}</span>
      )}
    </li>
  );
}

/** The menu for one editor; renders nothing while no mention is typed. */
export function MentionMenu({ store }: { store: MentionStore }) {
  const session = useStore(store, (state) => state.session);
  const active = useStore(store, (state) => state.active);
  if (session?.rect == null) return null;
  const { rect, items, choose } = session;
  return createPortal(
    <div
      className="fixed z-(--z-popover) max-h-[360px] w-64 overflow-y-auto rounded-lg bg-surface-raised p-1 shadow-(--shadow-popover)"
      style={{ top: rect.bottom + MENU_OFFSET, left: rect.left }}
    >
      {items.length === 0 ? (
        <p className="px-2 py-1.5 text-sm text-fg-muted">{notepadCopy.noPeople}</p>
      ) : (
        <ul role="listbox" aria-label={notepadCopy.mentionsLabel}>
          {items.map((candidate, index) => (
            <MentionOption
              key={candidate.id}
              candidate={candidate}
              active={index === active}
              onChoose={() => {
                choose(candidate);
              }}
            />
          ))}
        </ul>
      )}
    </div>,
    document.body,
  );
}
