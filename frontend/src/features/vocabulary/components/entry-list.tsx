/**
 * One kind of vocabulary entry as a settings card: the composer on top,
 * then each entry as a row that turns into the composer to edit it.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { IconButton } from '@/ui/icon-button';
import { ArrowRight, ICON_PROPS, Pencil, Trash2 } from '@/ui/icons';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';

import { vocabularyCopy as copy } from '../copy';
import { useDeleteTerm } from '../queries';
import type { VocabKind, VocabTerm, VocabTermId } from '../types';
import { EntryComposer } from './entry-composer';

/** What each kind's card is called and says when empty. */
const SECTIONS = {
  term: {
    title: copy.termsTitle,
    description: copy.termsDescription,
    empty: copy.termsEmpty,
  },
  replacement: {
    title: copy.replacementsTitle,
    description: copy.replacementsDescription,
    empty: copy.replacementsEmpty,
  },
} as const;

/** A term's spelling and what it sounds like. */
function TermText({ entry }: { entry: VocabTerm }) {
  return (
    <div className="min-w-0">
      <p className="truncate text-sm font-semibold text-fg">{entry.text}</p>
      <p className="mt-0.5 truncate text-xs text-fg-muted">
        {entry.soundsLike.length === 0 ? copy.spellingOnly : copy.soundsLike(entry.soundsLike)}
      </p>
    </div>
  );
}

/** A replacement: the phrases it replaces, then what they become. */
function ReplacementText({ entry }: { entry: VocabTerm }) {
  return (
    <p className="flex min-w-0 items-center gap-2 text-sm">
      <span className="truncate text-fg-muted">{entry.soundsLike.join(', ')}</span>
      <ArrowRight {...ICON_PROPS} size={14} className="shrink-0 text-fg-subtle" />
      <span className="truncate font-semibold text-fg">{entry.text}</span>
    </p>
  );
}

/** One entry, with Edit and Remove shown on hover or focus. */
function EntryRow(props: { entry: VocabTerm; onEdit: () => void; onRemove: () => void }) {
  const { entry } = props;
  return (
    <div className="group grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-2.5">
      {entry.kind === 'term' ? <TermText entry={entry} /> : <ReplacementText entry={entry} />}
      <div className="flex gap-0.5 opacity-0 transition-opacity duration-(--dur-fast) ease-out group-focus-within:opacity-100 group-hover:opacity-100">
        <IconButton icon={Pencil} label={copy.edit(entry.text)} onClick={props.onEdit} />
        <IconButton icon={Trash2} label={copy.remove(entry.text)} onClick={props.onRemove} />
      </div>
    </div>
  );
}

/** Removes an entry, saying so when it cannot. */
function useRemove() {
  const remove = useDeleteTerm();
  return (id: VocabTermId) => {
    remove.mutate(id, {
      onError: (error) => toast.error(copy.removeFailed, { description: error.message }),
    });
  };
}

/** Props of {@link EntryList}. */
export interface EntryListProps {
  kind: VocabKind;
  entries: readonly VocabTerm[];
}

/** The card of one kind of entry. */
export function EntryList({ kind, entries }: EntryListProps) {
  const [editing, setEditing] = useState<VocabTermId | null>(null);
  const remove = useRemove();
  const section = SECTIONS[kind];
  const stopEditing = () => {
    setEditing(null);
  };
  return (
    <SettingsSection title={section.title} description={section.description}>
      <SettingsCard label={section.title}>
        <EntryComposer kind={kind} />
        {entries.length === 0 ? (
          <p className="px-4 py-3 text-xs text-fg-muted">{section.empty}</p>
        ) : null}
        {entries.map((entry) =>
          editing === entry.id ? (
            <EntryComposer key={entry.id} kind={kind} editing={entry} onDone={stopEditing} />
          ) : (
            <EntryRow
              key={entry.id}
              entry={entry}
              onEdit={() => {
                setEditing(entry.id);
              }}
              onRemove={() => {
                remove(entry.id);
              }}
            />
          ),
        )}
      </SettingsCard>
    </SettingsSection>
  );
}
