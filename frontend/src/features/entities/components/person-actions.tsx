/**
 * The person page's actions: Edit, Email (when they have an address) and
 * the ⋯ menu (merge a duplicate, copy the address), with the dialogs they
 * open.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { Button, buttonVariants } from '@/ui/button';
import { Copy, Ellipsis, GitMerge, ICON_PROPS, Mail, Pencil } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Hint } from '@/ui/tooltip';

import { entitiesCopy } from '../copy';
import { useMergePeople } from '../mutations';
import { usePeople } from '../queries';
import { asPersonId, type Person } from '../types';
import { MergeDialog } from './merge-dialog';
import { PersonDialog } from './person-dialog';

/** Merges a duplicate into `person`, choosing from everyone else. */
function MergePeopleDialog({
  person,
  open,
  onClose,
}: {
  person: Person;
  open: boolean;
  onClose: () => void;
}) {
  const people = usePeople().data ?? [];
  const merge = useMergePeople();
  const candidates = people
    .filter((row) => row.id !== person.id)
    .map((row) => ({
      id: row.id,
      name: row.name,
      detail: row.email,
      logoPath: row.companyLogoPath,
    }));
  return (
    <MergeDialog
      open={open}
      onClose={onClose}
      kind="person"
      name={person.name}
      candidates={candidates}
      pending={merge.isPending}
      merge={(absorbed) =>
        merge.mutateAsync({ survivor: person.id, absorbed: asPersonId(absorbed) })
      }
    />
  );
}

/** The ⋯ menu. */
function PersonMenu({ person, onMerge }: { person: Person; onMerge: () => void }) {
  const email = person.email;
  return (
    <DropdownMenu>
      <Hint label={entitiesCopy.moreActions}>
        <DropdownMenuTrigger
          aria-label={entitiesCopy.moreActions}
          className={buttonVariants({ variant: 'ghost', size: 'icon' })}
        >
          <Ellipsis {...ICON_PROPS} />
        </DropdownMenuTrigger>
      </Hint>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={onMerge}>
          <GitMerge {...ICON_PROPS} />
          {entitiesCopy.merge}
        </DropdownMenuItem>
        {email === null ? null : (
          <DropdownMenuItem
            onSelect={() => {
              void navigator.clipboard
                .writeText(email)
                .then(() => toast.success(entitiesCopy.copied));
            }}
          >
            <Copy {...ICON_PROPS} />
            {entitiesCopy.copyEmail}
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Email them, when they have an address. */
function EmailButton({ email }: { email: string | null }) {
  if (email === null) return null;
  return (
    <Hint label={entitiesCopy.email}>
      <a
        href={`mailto:${email}`}
        aria-label={entitiesCopy.email}
        className={buttonVariants({ variant: 'ghost', size: 'icon' })}
      >
        <Mail {...ICON_PROPS} />
      </a>
    </Hint>
  );
}

/** Edit, Email and ⋯, with their dialogs. */
export function PersonActions({ person }: { person: Person }) {
  const [open, setOpen] = useState<'edit' | 'merge' | null>(null);
  const close = () => {
    setOpen(null);
  };
  return (
    <>
      <EmailButton email={person.email} />
      <Button
        variant="secondary"
        size="sm"
        onClick={() => {
          setOpen('edit');
        }}
      >
        <Pencil {...ICON_PROPS} />
        {entitiesCopy.edit}
      </Button>
      <PersonMenu
        person={person}
        onMerge={() => {
          setOpen('merge');
        }}
      />
      <PersonDialog target={open === 'edit' ? { kind: 'edit', person } : null} onClose={close} />
      <MergePeopleDialog person={person} open={open === 'merge'} onClose={close} />
    </>
  );
}
