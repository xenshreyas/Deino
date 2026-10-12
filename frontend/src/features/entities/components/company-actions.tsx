/**
 * The company page's actions: Edit and the ⋯ menu (merge a duplicate,
 * fetch the logo), with the dialogs they open. A logo fetch the network
 * settings refuse says where to allow it.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { IpcError } from '@/lib/ipc/result';
import { Button, buttonVariants } from '@/ui/button';
import { Ellipsis, GitMerge, ICON_PROPS, ImageDown, Pencil } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Hint } from '@/ui/tooltip';

import { entitiesCopy } from '../copy';
import { useFetchLogo, useMergeCompanies } from '../mutations';
import { useCompanies } from '../queries';
import { asCompanyId, type Company } from '../types';
import { CompanyDialog } from './company-dialog';
import { MergeDialog } from './merge-dialog';

/** Merges a duplicate into `company`, choosing from every other company. */
function MergeCompaniesDialog({
  company,
  open,
  onClose,
}: {
  company: Company;
  open: boolean;
  onClose: () => void;
}) {
  const companies = useCompanies().data ?? [];
  const merge = useMergeCompanies();
  const candidates = companies
    .filter((row) => row.id !== company.id)
    .map((row) => ({ id: row.id, name: row.name, detail: row.domain, logoPath: row.logoPath }));
  return (
    <MergeDialog
      open={open}
      onClose={onClose}
      kind="company"
      name={company.name}
      candidates={candidates}
      pending={merge.isPending}
      merge={(absorbed) =>
        merge.mutateAsync({ survivor: company.id, absorbed: asCompanyId(absorbed) })
      }
    />
  );
}

/** Fetches the logo again and says what happened. */
function useLogoAction(company: Company) {
  const fetchLogo = useFetchLogo();
  return () => {
    fetchLogo.mutate(
      { id: company.id, refresh: true },
      {
        // A new logo shows on the page; only "no logo" needs saying.
        onSuccess: (path) => {
          if (path === null) toast(entitiesCopy.logoNone);
        },
        onError: (error) => {
          const refused = error instanceof IpcError && error.code === 'permission_denied';
          if (refused) toast(entitiesCopy.logoOff);
          else toast.error(entitiesCopy.actionFailed, { description: error.message });
        },
      },
    );
  };
}

/** The ⋯ menu. */
function CompanyMenu({ company, onMerge }: { company: Company; onMerge: () => void }) {
  const fetchLogo = useLogoAction(company);
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
        {company.domain === null ? null : (
          <DropdownMenuItem onSelect={fetchLogo}>
            <ImageDown {...ICON_PROPS} />
            {company.logoCheckedAt === null ? entitiesCopy.fetchLogo : entitiesCopy.refetchLogo}
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Edit and ⋯, with their dialogs. */
export function CompanyActions({ company }: { company: Company }) {
  const [open, setOpen] = useState<'edit' | 'merge' | null>(null);
  const close = () => {
    setOpen(null);
  };
  return (
    <>
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
      <CompanyMenu
        company={company}
        onMerge={() => {
          setOpen('merge');
        }}
      />
      <CompanyDialog target={open === 'edit' ? { kind: 'edit', company } : null} onClose={close} />
      <MergeCompaniesDialog company={company} open={open === 'merge'} onClose={close} />
    </>
  );
}
