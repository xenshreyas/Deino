/**
 * The "Edit with AI" command bar (NOT-06) above the newest enhanced notes:
 * the request, the selection it is scoped to, then the proposed edit as a
 * diff with Accept and Reject. Accepting stores a new version; the toast's
 * Undo restores the previous one.
 */
import { type FormEvent, useState } from 'react';

import type { MeetingId } from '@/features/meetings';
import { useMountEffect } from '@/hooks/useMountEffect';
import { Button } from '@/ui/button';
import { CornerDownLeft, ICON_PROPS, LoaderCircle, Sparkles, X } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { editCopy } from './copy';
import { DiffPreview } from './diff-preview';
import { useAcceptEdit, useProposeEdit } from './hooks';
import { clearEditScope, closeEditBar, useEditBarStore } from './store';

/** Props of {@link EditBar}. */
export interface EditBarProps {
  meetingId: MeetingId;
  /** The newest version, the one the edit applies to. */
  version: number;
}

/** The bar, while it is open. */
export function EditBar({ meetingId, version }: EditBarProps) {
  const open = useEditBarStore((state) => state.open);
  useMountEffect(() => closeEditBar);
  if (!open) return null;
  return (
    <EditPanel key={`${meetingId}:${String(version)}`} meetingId={meetingId} version={version} />
  );
}

/** The request, then the proposal. */
function EditPanel({ meetingId, version }: EditBarProps) {
  const scope = useEditBarStore((state) => state.scope);
  const propose = useProposeEdit();
  const accept = useAcceptEdit(meetingId);
  const proposal = propose.data;
  return (
    <section
      aria-label={editCopy.button}
      className="mb-3 shrink-0 rounded-lg border border-border bg-surface shadow-xs"
    >
      <RequestForm
        pending={propose.isPending}
        blocks={scope.length}
        onSubmit={(instruction) => {
          propose.mutate({ meetingId, version, instruction, selection: scope });
        }}
      />
      <PanelStatus pending={propose.isPending} error={propose.error ?? accept.error} />
      {proposal === undefined ? null : (
        <DiffPreview
          proposal={proposal}
          accepting={accept.isPending}
          onAccept={() => {
            accept.mutate(proposal, { onSuccess: closeEditBar });
          }}
          onReject={() => {
            propose.reset();
          }}
        />
      )}
    </section>
  );
}

/** Props of {@link RequestForm}. */
interface RequestFormProps {
  pending: boolean;
  /** The selected blocks; 0 for the whole notes. */
  blocks: number;
  onSubmit: (instruction: string) => void;
}

/** The request field, its scope and Propose edit. */
function RequestForm({ pending, blocks, onSubmit }: RequestFormProps) {
  const [instruction, setInstruction] = useState('');
  const empty = instruction.trim() === '';
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!empty && !pending) onSubmit(instruction);
  };
  return (
    <form onSubmit={submit} className="flex items-center gap-2 px-3 py-2">
      <Sparkles {...ICON_PROPS} className="shrink-0 text-accent" />
      <input
        // The bar opens on a deliberate action (a click or ⌘⇧E), so it takes the focus.
        autoFocus
        aria-label={editCopy.inputLabel}
        placeholder={editCopy.placeholder}
        value={instruction}
        onChange={(event) => {
          setInstruction(event.target.value);
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') closeEditBar();
        }}
        className="h-8 min-w-0 flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-subtle"
      />
      <ScopeChip blocks={blocks} />
      <Button type="submit" variant="secondary" size="sm" disabled={empty || pending}>
        {pending ? (
          <LoaderCircle {...ICON_PROPS} className="animate-spin" />
        ) : (
          <CornerDownLeft {...ICON_PROPS} />
        )}
        {editCopy.submit}
      </Button>
      <IconButton icon={X} label={editCopy.close} onClick={closeEditBar} />
    </form>
  );
}

/** "2 selected lines ×", or "Whole notes". */
function ScopeChip({ blocks }: { blocks: number }) {
  if (blocks === 0) {
    return <span className="shrink-0 text-xs text-fg-subtle">{editCopy.wholeNotes}</span>;
  }
  return (
    <span className="inline-flex h-6 shrink-0 items-center gap-1 rounded-full bg-bg-muted pr-1 pl-2.5 text-xs text-fg">
      {editCopy.selection(blocks)}
      <button
        type="button"
        aria-label={editCopy.clearSelection}
        onClick={clearEditScope}
        className="inline-flex size-4 items-center justify-center rounded-full text-fg-muted hover:bg-bg-subtle hover:text-fg"
      >
        <X size={12} strokeWidth={1.5} aria-hidden />
      </button>
    </span>
  );
}

/** "Drafting the edit…", or why it failed. */
function PanelStatus({ pending, error }: { pending: boolean; error: Error | null }) {
  if (pending) {
    return (
      <p role="status" className="border-t border-border px-3 py-2 text-sm text-ai-text">
        {editCopy.drafting}
      </p>
    );
  }
  if (error === null) return null;
  return (
    <p role="alert" className="border-t border-border px-3 py-2 text-sm text-danger">
      {error.message}
    </p>
  );
}
