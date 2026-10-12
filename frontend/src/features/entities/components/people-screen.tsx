/**
 * The People page (`/people`, ORG-04): everyone the user meets, searchable,
 * most recently met first or by name, each row opening their page.
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { ICON_PROPS, Plus, Users } from '@/ui/icons';
import { Page, PageHeader } from '@/ui/page-header';

import { entitiesCopy } from '../copy';
import { relativeDay, visibleRows } from '../format';
import { usePeople } from '../queries';
import type { EntitySort, PersonRow } from '../types';
import { EntityAvatar } from './entity-avatar';
import { EntityRowBody, ListSkeleton, ListToolbar, NoMatches, ROW_CLASS } from './entity-list';
import { PersonDialog, type PersonDialogTarget } from './person-dialog';

/** A person row as the list searches and sorts it. */
function listable(row: PersonRow) {
  return { ...row, haystack: [row.email, row.companyName, row.title] };
}

/** `Title · Company`, either, or nothing. */
function detailOf(row: PersonRow): string {
  return [row.title, row.companyName].filter((part) => part !== null && part !== '').join(' · ');
}

/** One person: avatar, name, title and company, when last met and how often. */
function PersonListRow({ row, now }: { row: PersonRow; now: Date }) {
  const avatar = (
    <EntityAvatar id={row.id} name={row.name} kind="person" logoPath={row.companyLogoPath} />
  );
  return (
    <li>
      <Link to="/people/$personId" params={{ personId: row.id }} className={ROW_CLASS}>
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

/** The rows, or what to show instead. */
function PeopleList({
  rows,
  query,
  sort,
  onClear,
}: {
  rows: readonly PersonRow[];
  query: string;
  sort: EntitySort;
  onClear: () => void;
}) {
  const visible = visibleRows(rows.map(listable), query, sort);
  if (visible.length === 0) {
    return <NoMatches message={entitiesCopy.noMatches(query.trim())} onClear={onClear} />;
  }
  const now = new Date();
  return (
    <ul aria-label={entitiesCopy.people} className="flex flex-col">
      {visible.map((row) => (
        <PersonListRow key={row.id} row={row} now={now} />
      ))}
    </ul>
  );
}

/** Nobody yet: how they will appear, and the way to add one. */
function NoPeople({ onAdd }: { onAdd: () => void }) {
  return (
    <EmptyState
      className="pt-[12vh]"
      icon={Users}
      title={entitiesCopy.emptyPeopleTitle}
      description={entitiesCopy.emptyPeopleBody}
      action={
        <Button variant="secondary" onClick={onAdd}>
          {entitiesCopy.addPerson}
        </Button>
      }
    />
  );
}

/** The page body: loading, empty, or the searchable list. */
function PeopleBody({ onAdd }: { onAdd: () => void }) {
  const people = usePeople();
  const [query, setQuery] = useState('');
  const [sort, setSort] = useState<EntitySort>('recent');
  if (people.isPending) return <ListSkeleton />;
  if (people.isError) return <EmptyState icon={Users} title={entitiesCopy.loadError} />;
  if (people.data.length === 0) return <NoPeople onAdd={onAdd} />;
  return (
    <>
      <ListToolbar
        query={query}
        onQueryChange={setQuery}
        sort={sort}
        onSortChange={setSort}
        placeholder={entitiesCopy.searchPeople}
      />
      <PeopleList
        rows={people.data}
        query={query}
        sort={sort}
        onClear={() => {
          setQuery('');
        }}
      />
    </>
  );
}

/** The People page. */
export function PeopleScreen() {
  const [dialog, setDialog] = useState<PersonDialogTarget>(null);
  const add = () => {
    setDialog({ kind: 'create' });
  };
  return (
    <Page>
      <PageHeader
        title={entitiesCopy.people}
        description={entitiesCopy.peopleDescription}
        actions={
          <Button variant="secondary" onClick={add}>
            <Plus {...ICON_PROPS} />
            {entitiesCopy.addPerson}
          </Button>
        }
      />
      <PeopleBody onAdd={add} />
      <PersonDialog
        target={dialog}
        onClose={() => {
          setDialog(null);
        }}
      />
    </Page>
  );
}
