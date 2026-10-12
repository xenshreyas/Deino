/**
 * Merging a duplicate into the open person or company: search the others,
 * pick one, confirm. The page stays on the survivor; Rust moves every link
 * in one transaction.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { Command, CommandEmpty, CommandInput, CommandItem, CommandList } from '@/ui/command';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/ui/dialog';

import { entitiesCopy } from '../copy';
import { EntityAvatar } from './entity-avatar';

/** One entry that could be the duplicate. */
export interface MergeCandidate {
  id: string;
  name: string;
  /** Email or domain, to tell namesakes apart. */
  detail: string | null;
  logoPath: string | null;
}

/** Props of {@link MergeDialog}. */
export interface MergeDialogProps {
  open: boolean;
  onClose: () => void;
  kind: 'person' | 'company';
  /** The survivor's name. */
  name: string;
  candidates: readonly MergeCandidate[];
  /** Merges the chosen duplicate; rejects with a user-safe error. */
  merge: (absorbedId: string) => Promise<unknown>;
  pending: boolean;
}

/** A candidate's avatar, name and address or domain. */
function CandidateLabel({
  kind,
  candidate,
}: {
  kind: MergeDialogProps['kind'];
  candidate: MergeCandidate;
}) {
  return (
    <>
      <EntityAvatar
        id={candidate.id}
        name={candidate.name}
        kind={kind}
        logoPath={candidate.logoPath}
        size={20}
      />
      <span className="truncate">{candidate.name}</span>
      <span className="ml-auto truncate pl-2 text-xs text-fg-subtle">{candidate.detail}</span>
    </>
  );
}

/** Merges the chosen duplicate, then closes; failures stay open with a toast. */
function useMergeChoice(props: MergeDialogProps) {
  const [chosen, setChosen] = useState<string | null>(null);
  const close = () => {
    setChosen(null);
    props.onClose();
  };
  const confirm = () => {
    if (chosen === null) return;
    props.merge(chosen).then(close, (error: unknown) => {
      const description = error instanceof Error ? error.message : undefined;
      toast.error(entitiesCopy.actionFailed, { description });
    });
  };
  return { chosen, setChosen, close, confirm };
}

/** The searchable list of possible duplicates. */
function CandidateList({
  kind,
  candidates,
  chosen,
  onChoose,
}: {
  kind: MergeDialogProps['kind'];
  candidates: readonly MergeCandidate[];
  chosen: string | null;
  onChoose: (id: string) => void;
}) {
  return (
    <Command className="rounded-lg border border-border">
      <CommandInput placeholder={entitiesCopy.mergeSearch} autoFocus />
      <CommandList className="max-h-64">
        <CommandEmpty>{entitiesCopy.nothingToMerge}</CommandEmpty>
        {candidates.map((candidate) => (
          <CommandItem
            key={candidate.id}
            value={`${candidate.name} ${candidate.detail ?? ''} ${candidate.id}`}
            onSelect={() => {
              onChoose(candidate.id);
            }}
            data-checked={chosen === candidate.id}
            className="data-[checked=true]:bg-accent-soft"
          >
            <CandidateLabel kind={kind} candidate={candidate} />
          </CommandItem>
        ))}
      </CommandList>
    </Command>
  );
}

/** Picks and merges a duplicate. */
export function MergeDialog(props: MergeDialogProps) {
  const { chosen, setChosen, close, confirm } = useMergeChoice(props);
  const body = props.kind === 'person' ? entitiesCopy.mergeBody : entitiesCopy.mergeCompanyBody;
  return (
    <Dialog
      open={props.open}
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{entitiesCopy.mergeTitle(props.name)}</DialogTitle>
          <DialogDescription>{body}</DialogDescription>
        </DialogHeader>
        <CandidateList
          kind={props.kind}
          candidates={props.candidates}
          chosen={chosen}
          onChoose={setChosen}
        />
        <DialogFooter>
          <Button variant="ghost" onClick={close}>
            {entitiesCopy.cancel}
          </Button>
          <Button disabled={chosen === null || props.pending} onClick={confirm}>
            {entitiesCopy.mergeConfirm}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
