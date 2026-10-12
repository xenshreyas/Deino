/**
 * The Companies page (`/companies`, ORG-04): every organisation behind the
 * user's meetings, searchable, most recently met first or by name.
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { Building2, ICON_PROPS, Plus } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';

import { entitiesCopy } from '../copy';
import { relativeDay, visibleRows } from '../format';
import { useCompanies } from '../queries';
import type { CompanyRow, EntitySort } from '../types';
import { CompanyDialog, type CompanyDialogTarget } from './company-dialog';
import { EntityAvatar } from './entity-avatar';
import { EntityTabs } from './entity-tabs';
import { EntityRowBody, ListSkeleton, ListToolbar, NoMatches, ROW_CLASS } from './entity-list';

/** A company row as the list searches and sorts it. */
function listable(row: CompanyRow) {
  return { ...row, haystack: [row.domain] };
}

/** `domain · 3 people`, or whichever part is known. */
function detailOf(row: CompanyRow): string {
  const people = row.peopleCount === 0 ? null : entitiesCopy.peopleCount(row.peopleCount);
  return [row.domain, people].filter((part) => part !== null).join(' · ');
}

/** One company: logo, name, domain and people, when last met and how often. */
function CompanyListRow({ row, now }: { row: CompanyRow; now: Date }) {
  const avatar = (
    <EntityAvatar id={row.id} name={row.name} kind="company" logoPath={row.logoPath} />
  );
  return (
    <li>
      <Link to="/companies/$companyId" params={{ companyId: row.id }} className={ROW_CLASS}>
        <EntityRowBody
          avatar={avatar}
          name={row.name}
          detail={detailOf(row)}
          when={row.lastMetAt === null ? entitiesCopy.neverMet : relativeDay(row.lastMetAt, now)}
          count={row.meetingCount === 0 ? '' : entitiesCopy.meetingCount(row.meetingCount)}
        />
      </Link>
    </li>
  );
}

/** The rows, or "no match". */
function CompanyList({
  rows,
  query,
  sort,
  onClear,
}: {
  rows: readonly CompanyRow[];
  query: string;
  sort: EntitySort;
  onClear: () => void;
}) {
  const visible = visibleRows(rows.map(listable), query, sort);
  if (visible.length === 0) {
    return <NoMatches message={entitiesCopy.noCompanyMatches(query.trim())} onClear={onClear} />;
  }
  const now = new Date();
  return (
    <ul aria-label={entitiesCopy.companies} className="flex flex-col">
      {visible.map((row) => (
        <CompanyListRow key={row.id} row={row} now={now} />
      ))}
    </ul>
  );
}

/** Nobody yet: how they will appear, and the way to add one. */
function NoCompanies({ onAdd }: { onAdd: () => void }) {
  return (
    <EmptyState
      className="pt-[12vh]"
      icon={Building2}
      title={entitiesCopy.emptyCompaniesTitle}
      description={entitiesCopy.emptyCompaniesBody}
      action={
        <Button variant="secondary" onClick={onAdd}>
          {entitiesCopy.addCompany}
        </Button>
      }
    />
  );
}

/** The page body: loading, empty, or the searchable list. */
function CompaniesBody({ onAdd }: { onAdd: () => void }) {
  const companies = useCompanies();
  const [query, setQuery] = useState('');
  const [sort, setSort] = useState<EntitySort>('recent');
  if (companies.isPending) return <ListSkeleton />;
  if (companies.isError) return <EmptyState icon={Building2} title={entitiesCopy.loadError} />;
  if (companies.data.length === 0) return <NoCompanies onAdd={onAdd} />;
  return (
    <>
      <ListToolbar
        query={query}
        onQueryChange={setQuery}
        sort={sort}
        onSortChange={setSort}
        placeholder={entitiesCopy.searchCompanies}
      />
      <CompanyList
        rows={companies.data}
        query={query}
        sort={sort}
        onClear={() => {
          setQuery('');
        }}
      />
    </>
  );
}

/** The Companies page. */
export function CompaniesScreen() {
  const [dialog, setDialog] = useState<CompanyDialogTarget>(null);
  const add = () => {
    setDialog({ kind: 'create' });
  };
  return (
    <Page>
      <PageHeader
        title={entitiesCopy.companies}
        description={entitiesCopy.companiesDescription}
        actions={
          <Button variant="secondary" onClick={add}>
            <Plus {...ICON_PROPS} />
            {entitiesCopy.addCompany}
          </Button>
        }
      />
      <EntityTabs />
      <CompaniesBody onAdd={add} />
      <CompanyDialog
        target={dialog}
        onClose={() => {
          setDialog(null);
        }}
      />
    </Page>
  );
}
